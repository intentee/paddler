use log::error;
use log::info;
use log::warn;
use tokio_util::sync::CancellationToken;

use paddler_agent_decision::decision_pipeline::DecisionPipeline;
use paddler_agent_embeddings::embedding_pipeline::EmbeddingPipeline;
use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_runtime::prepares_scheduler_command::PreparesSchedulerCommand;
use paddler_agent_runtime::scheduler_handle::SchedulerHandle;
use paddler_agent_runtime::scheduler_spawn_outcome::SchedulerSpawnOutcome;
use paddler_agent_text_generation::text_generation_pipeline::TextGenerationPipeline;
use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
use paddler_messaging::inference_mode::InferenceMode;

use crate::agent_applicable_state::AgentApplicableState;
use crate::agent_error::AgentError;
use crate::pipeline_request::PipelineRequest;
use crate::running_pipeline::RunningPipeline;

fn running_once_started<TPreparation, TError>(
    inference_runtime_context: &InferenceRuntimeContext,
    scheduler_spawn_outcome: SchedulerSpawnOutcome<TPreparation, TError>,
    serving_inference_mode: InferenceMode,
    running_pipeline: fn(SchedulerHandle<TPreparation, TError>) -> RunningPipeline,
) -> PipelineArbiterState
where
    TPreparation: PreparesSchedulerCommand,
{
    match scheduler_spawn_outcome {
        SchedulerSpawnOutcome::Ready(scheduler_handle) => {
            inference_runtime_context
                .slot_aggregated_status
                .start_serving(serving_inference_mode);

            info!("Reconciled state change applied successfully");

            PipelineArbiterState::Running(running_pipeline(scheduler_handle))
        }
        SchedulerSpawnOutcome::Cancelled => {
            info!("Model load was cancelled by shutdown before it finished");

            PipelineArbiterState::Idle
        }
    }
}

pub enum PipelineArbiterState {
    Idle,
    Running(RunningPipeline),
}

impl PipelineArbiterState {
    pub async fn apply(
        self,
        shutdown: &CancellationToken,
        agent_applicable_state: AgentApplicableState,
        inference_runtime_context: &InferenceRuntimeContext,
    ) -> Self {
        match self
            .try_to_apply(shutdown, agent_applicable_state, inference_runtime_context)
            .await
        {
            Ok(next_state) => next_state,
            Err(application_error) => {
                error!("Failed to apply reconciled state change: {application_error}");

                Self::Idle
            }
        }
    }

    pub fn forward(&self, agent_name: Option<&str>, request: PipelineRequest) {
        match self {
            Self::Idle => request.reject_because_no_model_is_loaded(agent_name),
            Self::Running(running_pipeline) => running_pipeline.forward(agent_name, request),
        }
    }

    pub async fn shut_down(self) -> Result<(), AgentError> {
        match self {
            Self::Idle => Ok(()),
            Self::Running(running_pipeline) => running_pipeline.shut_down().await,
        }
    }

    async fn try_to_apply(
        self,
        shutdown: &CancellationToken,
        agent_applicable_state: AgentApplicableState,
        inference_runtime_context: &InferenceRuntimeContext,
    ) -> Result<Self, AgentError> {
        inference_runtime_context.slot_aggregated_status.reset();
        inference_runtime_context
            .model_metadata_holder
            .forget_model_metadata();
        self.shut_down().await?;

        let next_state = match agent_applicable_state {
            AgentApplicableState::Decision {
                model_path,
                model_runtime_parameters,
                pointer_head_path,
            } => running_once_started(
                inference_runtime_context,
                DecisionPipeline {
                    inference_runtime_context: inference_runtime_context.clone(),
                    model_path,
                    model_runtime_parameters,
                    pointer_head_path,
                }
                .spawn(shutdown)
                .await?,
                InferenceMode::Decision,
                RunningPipeline::Decision,
            ),
            AgentApplicableState::Embeddings {
                embedding_parameters,
                model_path,
                model_runtime_parameters,
            } => running_once_started(
                inference_runtime_context,
                EmbeddingPipeline {
                    embedding_parameters,
                    inference_runtime_context: inference_runtime_context.clone(),
                    model_path,
                    model_runtime_parameters,
                }
                .spawn(shutdown)
                .await?,
                InferenceMode::Embeddings,
                RunningPipeline::Embeddings,
            ),
            AgentApplicableState::NotConfigured
            | AgentApplicableState::TextGenerationWithoutModel { .. } => {
                warn!("No model configured in applicable state; skipping llama.cpp initialization");

                Self::Idle
            }
            AgentApplicableState::TextGeneration {
                model_path,
                model_runtime_parameters,
                text_generation_settings,
            } => running_once_started(
                inference_runtime_context,
                TextGenerationPipeline {
                    inference_runtime_context: inference_runtime_context.clone(),
                    model_path,
                    model_runtime_parameters,
                    text_generation_settings,
                }
                .spawn(shutdown)
                .await?,
                InferenceMode::TextGeneration,
                RunningPipeline::TextGeneration,
            ),
        };

        inference_runtime_context
            .slot_aggregated_status
            .set_state_application_status(AgentStateApplicationStatus::Applied);

        Ok(next_state)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::mem::discriminant;
    use std::num::NonZeroU32;
    use std::sync::Arc;

    use tokio::sync::mpsc;
    use tokio_util::sync::CancellationToken;

    use paddler_agent_runtime::agent_request::AgentRequest;
    use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
    use paddler_agent_runtime::model_metadata_holder::ModelMetadataHolder;
    use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
    use paddler_agent_status::slot_guard::SlotGuard;
    use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use paddler_messaging::model_metadata::ModelMetadata;
    use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;

    use super::PipelineArbiterState;
    use crate::agent_applicable_state::AgentApplicableState;
    use crate::pipeline_request::PipelineRequest;

    fn inference_runtime_context() -> InferenceRuntimeContext {
        InferenceRuntimeContext {
            agent_name: Some("agent".to_owned()),
            model_metadata_holder: Arc::new(ModelMetadataHolder::default()),
            slot_aggregated_status: Arc::new(SlotAggregatedStatus::new(1)),
        }
    }

    async fn applied(
        agent_applicable_state: AgentApplicableState,
        inference_runtime_context: &InferenceRuntimeContext,
    ) -> PipelineArbiterState {
        PipelineArbiterState::Idle
            .apply(
                &CancellationToken::new(),
                agent_applicable_state,
                inference_runtime_context,
            )
            .await
    }

    #[tokio::test]
    async fn an_idle_arbiter_rejects_a_request_because_no_model_is_loaded() {
        let inference_runtime_context = inference_runtime_context();
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();
        let (_generate_tokens_stop_tx, generate_tokens_stop_rx) = mpsc::unbounded_channel();

        PipelineArbiterState::Idle.forward(
            inference_runtime_context.agent_name.as_deref(),
            PipelineRequest::ContinueFromRawPrompt(AgentRequest {
                params: ContinueFromRawPromptParams {
                    grammar: None,
                    max_tokens: NonZeroU32::MIN,
                    raw_prompt: "Hello".to_owned(),
                },
                response_tx: generated_tokens_tx,
                slot_guard: SlotGuard::new(
                    inference_runtime_context.slot_aggregated_status.clone(),
                ),
                stop_rx: generate_tokens_stop_rx,
            }),
        );

        assert_eq!(
            generated_tokens_rx.recv().await,
            Some(GeneratedTokenResult::ModelNotLoaded(
                "agent: no model is loaded".to_owned()
            ))
        );
        assert_eq!(
            inference_runtime_context
                .slot_aggregated_status
                .slots_processing_count(),
            0
        );
    }

    #[tokio::test]
    async fn applying_a_state_without_a_model_stays_idle_and_reports_applied() {
        let inference_runtime_context = inference_runtime_context();

        let arbiter_state =
            applied(AgentApplicableState::default(), &inference_runtime_context).await;

        assert_eq!(
            discriminant(&arbiter_state),
            discriminant(&PipelineArbiterState::Idle)
        );
        assert_eq!(
            inference_runtime_context
                .slot_aggregated_status
                .get_state_application_status(),
            AgentStateApplicationStatus::Applied
        );
    }

    #[tokio::test]
    async fn applying_a_state_forgets_the_metadata_of_the_previous_model() {
        let inference_runtime_context = inference_runtime_context();

        inference_runtime_context
            .model_metadata_holder
            .set_model_metadata(ModelMetadata {
                metadata: BTreeMap::from([("general.name".to_owned(), "previous".to_owned())]),
            });

        applied(AgentApplicableState::default(), &inference_runtime_context).await;

        assert_eq!(
            inference_runtime_context
                .model_metadata_holder
                .get_model_metadata(),
            None
        );
    }
}
