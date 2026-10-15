use std::mem;
use std::sync::Arc;

use log::error;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;

use crate::agent_applicable_state::AgentApplicableState;
use crate::agent_applicable_state_holder::AgentApplicableStateHolder;
use crate::finished_pipeline_startup::FinishedPipelineStartup;
use crate::pipeline_arbiter_params::PipelineArbiterParams;
use crate::pipeline_request::PipelineRequest;
use crate::pipeline_startup::PipelineStartup;
use crate::retry_delay::RetryDelay;
use crate::serving_pipeline::ServingPipeline;
use crate::started_pipeline::StartedPipeline;

pub struct PipelineArbiter {
    agent_shutdown: CancellationToken,
    inference_runtime_context: InferenceRuntimeContext,
    pipeline_startup: PipelineStartup,
    served_applicable_state_holder: Arc<AgentApplicableStateHolder>,
    serving_pipeline: ServingPipeline,
    task_tracker: TaskTracker,
}

impl PipelineArbiter {
    #[must_use]
    pub fn new(
        PipelineArbiterParams {
            agent_shutdown,
            inference_runtime_context,
            served_applicable_state_holder,
        }: PipelineArbiterParams,
    ) -> Self {
        Self {
            agent_shutdown,
            inference_runtime_context,
            pipeline_startup: PipelineStartup::Idle,
            served_applicable_state_holder,
            serving_pipeline: ServingPipeline::Idle,
            task_tracker: TaskTracker::new(),
        }
    }

    pub fn apply(&mut self, agent_applicable_state: AgentApplicableState) {
        self.pipeline_startup.abandon(&self.task_tracker);
        self.inference_runtime_context
            .slot_aggregated_status
            .begin_pipeline();
        self.pipeline_startup = PipelineStartup::start(
            agent_applicable_state,
            &self.agent_shutdown,
            self.inference_runtime_context.clone(),
            RetryDelay::Immediately,
        );
    }

    pub async fn next_finished_startup(&mut self) -> FinishedPipelineStartup {
        self.pipeline_startup.finish().await
    }

    pub fn conclude(
        &mut self,
        FinishedPipelineStartup {
            agent_applicable_state,
            outcome,
        }: FinishedPipelineStartup,
    ) {
        let slot_aggregated_status = &self.inference_runtime_context.slot_aggregated_status;
        let uses_chat_template_override = agent_applicable_state.chat_template_override().is_some();

        match outcome {
            Ok(StartedPipeline::Running {
                model_path,
                running_pipeline,
            }) => {
                slot_aggregated_status.start_serving(
                    running_pipeline.inference_mode(),
                    model_path.model_path,
                    uses_chat_template_override,
                );
                self.inference_runtime_context
                    .model_metadata_holder
                    .set_model_metadata(running_pipeline.model_metadata());
                self.served_applicable_state_holder
                    .set_agent_applicable_state(agent_applicable_state);
                self.replace_serving_pipeline(ServingPipeline::Running(running_pipeline));
            }
            Ok(StartedPipeline::NothingToServe) => {
                slot_aggregated_status.serve_nothing(uses_chat_template_override);
                self.inference_runtime_context
                    .model_metadata_holder
                    .forget_model_metadata();
                self.served_applicable_state_holder
                    .set_agent_applicable_state(agent_applicable_state);
                self.replace_serving_pipeline(ServingPipeline::Idle);
            }
            Ok(StartedPipeline::Cancelled) => {}
            Err(startup_error) => {
                error!("Failed to start the pipeline of the reconciled state: {startup_error}");

                slot_aggregated_status.retry_pipeline();
                self.pipeline_startup = PipelineStartup::start(
                    agent_applicable_state,
                    &self.agent_shutdown,
                    self.inference_runtime_context.clone(),
                    RetryDelay::AfterRetryInterval,
                );
            }
        }
    }

    pub fn forward(&self, request: PipelineRequest) {
        self.serving_pipeline.forward(
            self.inference_runtime_context.agent_name.as_deref(),
            request,
        );
    }

    pub async fn shut_down(mut self) {
        self.pipeline_startup.abandon(&self.task_tracker);
        self.serving_pipeline.shut_down().await;
        self.task_tracker.close();
        self.task_tracker.wait().await;
    }

    fn replace_serving_pipeline(&mut self, serving_pipeline: ServingPipeline) {
        mem::replace(&mut self.serving_pipeline, serving_pipeline)
            .shut_down_in_background(&self.task_tracker);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::num::NonZeroU32;
    use std::path::PathBuf;
    use std::sync::Arc;

    use tokio_util::sync::CancellationToken;

    use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
    use paddler_agent_runtime::model_metadata_holder::ModelMetadataHolder;
    use paddler_agent_runtime::shared_llama_backend::SharedLlamaBackend;
    use paddler_agent_runtime::system_parallelism::SystemParallelism;
    use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
    use paddler_agent_text_generation::multimodal_projection::MultimodalProjection;
    use paddler_agent_text_generation::text_generation_settings::TextGenerationSettings;
    use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
    use paddler_inference_parameters::sampling_parameters::SamplingParameters;
    use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
    use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
    use paddler_messaging::chat_template::ChatTemplate;
    use paddler_messaging::chat_template_source::ChatTemplateSource;
    use paddler_messaging::model_metadata::ModelMetadata;
    use paddler_messaging::produces_snapshot::ProducesSnapshot as _;

    use super::PipelineArbiter;
    use crate::agent_applicable_state::AgentApplicableState;
    use crate::agent_applicable_state_holder::AgentApplicableStateHolder;
    use crate::pipeline_arbiter_params::PipelineArbiterParams;

    fn inference_runtime_context() -> InferenceRuntimeContext {
        InferenceRuntimeContext {
            agent_name: Some("agent".to_owned()),
            available_parallelism: Arc::new(SystemParallelism),
            llama_backend: Arc::new(SharedLlamaBackend::default()),
            model_metadata_holder: Arc::new(ModelMetadataHolder::default()),
            slot_aggregated_status: Arc::new(SlotAggregatedStatus::new(1)),
        }
    }

    fn pipeline_arbiter(
        inference_runtime_context: InferenceRuntimeContext,
        served_applicable_state_holder: Arc<AgentApplicableStateHolder>,
    ) -> PipelineArbiter {
        PipelineArbiter::new(PipelineArbiterParams {
            agent_shutdown: CancellationToken::new(),
            inference_runtime_context,
            served_applicable_state_holder,
        })
    }

    fn state_whose_pipeline_cannot_start() -> AgentApplicableState {
        AgentApplicableState::TextGeneration {
            model_path: PathBuf::from("/nonexistent/model.gguf"),
            model_runtime_parameters: ModelRuntimeParameters::default(),
            text_generation_settings: TextGenerationSettings {
                chat_template_source: ChatTemplateSource::EmbeddedInModel,
                image_resize_to_fit: NonZeroU32::MIN,
                multimodal_projection: MultimodalProjection::NotConfigured,
                sampling_parameters: SamplingParameters::default(),
            },
        }
    }

    async fn concluded_startup(
        pipeline_arbiter: &mut PipelineArbiter,
    ) -> AgentStateApplicationStatus {
        let finished_pipeline_startup = pipeline_arbiter.next_finished_startup().await;

        pipeline_arbiter.conclude(finished_pipeline_startup);

        pipeline_arbiter
            .inference_runtime_context
            .slot_aggregated_status
            .get_state_application_status()
    }

    #[tokio::test]
    async fn a_state_without_a_model_serves_nothing_and_forgets_the_previous_model() {
        let inference_runtime_context = inference_runtime_context();
        let mut pipeline_arbiter = pipeline_arbiter(
            inference_runtime_context.clone(),
            Arc::new(AgentApplicableStateHolder::default()),
        );

        inference_runtime_context
            .model_metadata_holder
            .set_model_metadata(ModelMetadata {
                metadata: BTreeMap::from([("general.name".to_owned(), "previous".to_owned())]),
            });
        pipeline_arbiter.apply(AgentApplicableState::NotConfigured);

        assert_eq!(
            concluded_startup(&mut pipeline_arbiter).await,
            AgentStateApplicationStatus::Applied
        );
        assert_eq!(
            inference_runtime_context
                .slot_aggregated_status
                .make_snapshot()
                .status
                .runtime,
            AgentRuntimeStatus::Idle
        );
        assert_eq!(
            inference_runtime_context
                .model_metadata_holder
                .get_model_metadata(),
            None
        );

        pipeline_arbiter.shut_down().await;
    }

    #[tokio::test]
    async fn a_pipeline_that_fails_to_start_is_retried_until_it_is_stuck() {
        let mut pipeline_arbiter = pipeline_arbiter(
            inference_runtime_context(),
            Arc::new(AgentApplicableStateHolder::default()),
        );

        pipeline_arbiter.apply(state_whose_pipeline_cannot_start());

        assert_eq!(
            concluded_startup(&mut pipeline_arbiter).await,
            AgentStateApplicationStatus::AttemptedAndRetrying
        );
        assert_eq!(
            concluded_startup(&mut pipeline_arbiter).await,
            AgentStateApplicationStatus::Stuck
        );

        pipeline_arbiter.shut_down().await;
    }

    #[tokio::test]
    async fn a_pipeline_that_fails_to_start_leaves_the_previously_served_state_in_place() {
        let served_applicable_state_holder = Arc::new(AgentApplicableStateHolder::default());
        let mut pipeline_arbiter = pipeline_arbiter(
            inference_runtime_context(),
            served_applicable_state_holder.clone(),
        );
        let served_state = AgentApplicableState::TextGenerationWithoutModel {
            chat_template_source: ChatTemplateSource::Override(ChatTemplate {
                content: "{{ messages }}".to_owned(),
            }),
        };

        pipeline_arbiter.apply(served_state.clone());
        concluded_startup(&mut pipeline_arbiter).await;
        pipeline_arbiter.apply(state_whose_pipeline_cannot_start());
        concluded_startup(&mut pipeline_arbiter).await;

        assert_eq!(
            served_applicable_state_holder.get_agent_applicable_state(),
            served_state
        );

        pipeline_arbiter.shut_down().await;
    }

    #[tokio::test]
    async fn a_startup_cancelled_by_the_agent_shutting_down_changes_nothing() {
        let agent_shutdown = CancellationToken::new();
        let mut pipeline_arbiter = PipelineArbiter::new(PipelineArbiterParams {
            agent_shutdown: agent_shutdown.clone(),
            inference_runtime_context: inference_runtime_context(),
            served_applicable_state_holder: Arc::new(AgentApplicableStateHolder::default()),
        });

        agent_shutdown.cancel();
        pipeline_arbiter.apply(state_whose_pipeline_cannot_start());

        assert_eq!(
            concluded_startup(&mut pipeline_arbiter).await,
            AgentStateApplicationStatus::Fresh
        );

        pipeline_arbiter.shut_down().await;
    }
}
