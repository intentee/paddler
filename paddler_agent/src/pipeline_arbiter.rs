use std::mem;
use std::sync::Arc;

use log::error;
use tokio_util::sync::CancellationToken;

use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_runtime::shuts_down::ShutsDown as _;
use paddler_error_chain::error_chain::ErrorChain;

use crate::agent_applicable_state::AgentApplicableState;
use crate::agent_applicable_state_holder::AgentApplicableStateHolder;
use crate::finished_serving_startup::FinishedServingStartup;
use crate::model_serving::ModelServing;
use crate::pipeline_arbiter_params::PipelineArbiterParams;
use crate::pipeline_request::PipelineRequest;
use crate::replacement_outcome::ReplacementOutcome;
use crate::retry_delay::RetryDelay;
use crate::served_model::ServedModel;
use crate::serving_replacement::ServingReplacement;
use crate::serving_startup::ServingStartup;
use crate::startup_phase::StartupPhase;
use crate::superseding_state::SupersedingState;

pub struct PipelineArbiter {
    agent_shutdown: CancellationToken,
    inference_runtime_context: InferenceRuntimeContext,
    model_serving: ModelServing,
    served_applicable_state_holder: Arc<AgentApplicableStateHolder>,
    serving_startup: ServingStartup,
    superseding_state: SupersedingState,
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
            model_serving: ModelServing::Idle,
            served_applicable_state_holder,
            serving_startup: ServingStartup::Idle,
            superseding_state: SupersedingState::NotSuperseded,
        }
    }

    pub fn apply(&mut self, agent_applicable_state: AgentApplicableState) {
        match self.serving_startup {
            ServingStartup::Idle => self.start_pipeline(agent_applicable_state),
            ServingStartup::Replacing(_) => {
                self.inference_runtime_context
                    .slot_aggregated_status
                    .begin_pipeline();
                self.serving_startup.cancel();
                self.superseding_state = SupersedingState::SupersededBy(agent_applicable_state);
            }
        }
    }

    pub fn conclude(
        &mut self,
        FinishedServingStartup {
            agent_applicable_state,
            outcome,
        }: FinishedServingStartup,
    ) {
        if let ReplacementOutcome::Failed { error, .. } = &outcome {
            error!(
                "Failed to serve the reconciled state: {}",
                ErrorChain { error }
            );
        }

        if let SupersedingState::SupersededBy(superseding_applicable_state) =
            mem::replace(&mut self.superseding_state, SupersedingState::NotSuperseded)
        {
            self.model_serving = outcome.into_model_serving();
            self.start_pipeline(superseding_applicable_state);

            return;
        }

        self.settle_replacement(agent_applicable_state, &outcome);
        self.model_serving = outcome.into_model_serving();
    }

    pub fn forward(&self, request: PipelineRequest) {
        self.model_serving.forward(
            self.inference_runtime_context.agent_name.as_deref(),
            request,
        );
    }

    pub async fn next_finished_startup(&mut self) -> FinishedServingStartup {
        self.serving_startup.next_finished_startup().await
    }

    pub async fn shut_down(self) {
        let Self {
            model_serving,
            serving_startup,
            ..
        } = self;

        serving_startup.shut_down().await;
        model_serving.shut_down().await;
    }

    fn replace(&mut self, agent_applicable_state: AgentApplicableState, retry_delay: RetryDelay) {
        if let ModelServing::Serving(served_model) = &self.model_serving
            && let AgentApplicableState::WithModel(desired_settings) = &agent_applicable_state
            && served_model.model_serving_settings == *desired_settings
        {
            self.resume_serving(served_model, agent_applicable_state);

            return;
        }

        let held = self.withdraw_serving();
        let cancellation_token = self.agent_shutdown.child_token();

        self.serving_startup = ServingStartup::Replacing(StartupPhase {
            cancellation_token: cancellation_token.clone(),
            outcome: Box::pin(
                ServingReplacement {
                    agent_applicable_state,
                    held,
                    retry_delay,
                }
                .replace(cancellation_token, self.inference_runtime_context.clone()),
            ),
        });
    }

    fn resume_serving(
        &self,
        served_model: &ServedModel,
        agent_applicable_state: AgentApplicableState,
    ) {
        self.inference_runtime_context
            .slot_aggregated_status
            .start_serving(
                served_model
                    .serving_report(agent_applicable_state.chat_template_override().is_some()),
            );
        self.inference_runtime_context
            .model_metadata_holder
            .set_model_metadata(served_model.loaded_llama_model.model_metadata.clone());
        self.served_applicable_state_holder
            .set_agent_applicable_state(agent_applicable_state);
    }

    fn settle_replacement(
        &mut self,
        agent_applicable_state: AgentApplicableState,
        outcome: &ReplacementOutcome,
    ) {
        match outcome {
            ReplacementOutcome::Cancelled { .. } => {}
            ReplacementOutcome::Failed { .. } => {
                self.inference_runtime_context
                    .slot_aggregated_status
                    .retry_pipeline();
                self.replace(agent_applicable_state, RetryDelay::AfterRetryInterval);
            }
            ReplacementOutcome::NothingToServe => {
                self.inference_runtime_context
                    .slot_aggregated_status
                    .serve_nothing(agent_applicable_state.chat_template_override().is_some());
                self.served_applicable_state_holder
                    .set_agent_applicable_state(agent_applicable_state);
            }
            ReplacementOutcome::Serving(served_model) => {
                self.resume_serving(served_model, agent_applicable_state);
            }
        }
    }

    fn start_pipeline(&mut self, agent_applicable_state: AgentApplicableState) {
        self.inference_runtime_context
            .slot_aggregated_status
            .begin_pipeline();
        self.replace(agent_applicable_state, RetryDelay::Immediately);
    }

    fn withdraw_serving(&mut self) -> ModelServing {
        self.inference_runtime_context
            .slot_aggregated_status
            .withdraw_serving();
        self.inference_runtime_context
            .model_metadata_holder
            .forget_model_metadata();
        self.served_applicable_state_holder
            .set_agent_applicable_state(AgentApplicableState::NotConfigured);

        mem::replace(&mut self.model_serving, ModelServing::Idle)
    }
}
