use std::mem;
use std::sync::Arc;

use log::error;
use tokio_util::sync::CancellationToken;

use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_runtime::shuts_down::ShutsDown as _;

use crate::agent_applicable_state::AgentApplicableState;
use crate::agent_applicable_state_holder::AgentApplicableStateHolder;
use crate::finished_serving_startup::FinishedServingStartup;
use crate::model_serving::ModelServing;
use crate::pipeline_arbiter_params::PipelineArbiterParams;
use crate::pipeline_request::PipelineRequest;
use crate::preparation_outcome::PreparationOutcome;
use crate::prepared_serving::PreparedServing;
use crate::replacement_outcome::ReplacementOutcome;
use crate::retry_delay::RetryDelay;
use crate::served_model::ServedModel;
use crate::serving_preparation::ServingPreparation;
use crate::serving_replacement::ServingReplacement;
use crate::serving_startup::ServingStartup;
use crate::serving_startup_progress::ServingStartupProgress;
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
            ServingStartup::Preparing(_) | ServingStartup::Replacing(_) => {
                self.inference_runtime_context
                    .slot_aggregated_status
                    .begin_pipeline();
                self.serving_startup.cancel();
                self.superseding_state = SupersedingState::SupersededBy(agent_applicable_state);
            }
        }
    }

    pub async fn next_finished_startup(&mut self) -> ServingStartupProgress {
        self.serving_startup.next_progress().await
    }

    pub fn conclude(&mut self, serving_startup_progress: ServingStartupProgress) {
        match serving_startup_progress {
            ServingStartupProgress::Prepared(preparation_outcome) => {
                self.conclude_preparation(preparation_outcome);
            }
            ServingStartupProgress::Replaced(finished_serving_startup) => {
                self.conclude_replacement(finished_serving_startup);
            }
        }
    }

    pub fn forward(&self, request: PipelineRequest) {
        self.model_serving.forward(
            self.inference_runtime_context.agent_name.as_deref(),
            request,
        );
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

    fn conclude_preparation(&mut self, preparation_outcome: PreparationOutcome) {
        if let PreparationOutcome::Failed { error, .. } = &preparation_outcome {
            error!("Failed to prepare the reconciled state: {error}");
        }

        if let SupersedingState::SupersededBy(superseding_applicable_state) =
            mem::replace(&mut self.superseding_state, SupersedingState::NotSuperseded)
        {
            self.start_pipeline(superseding_applicable_state);

            return;
        }

        match preparation_outcome {
            PreparationOutcome::Cancelled => {}
            PreparationOutcome::Failed {
                agent_applicable_state,
                ..
            } => {
                self.inference_runtime_context
                    .slot_aggregated_status
                    .retry_pipeline();
                self.prepare(agent_applicable_state, RetryDelay::AfterRetryInterval);
            }
            PreparationOutcome::Prepared(prepared_serving) => self.replace(prepared_serving),
        }
    }

    fn conclude_replacement(
        &mut self,
        FinishedServingStartup {
            agent_applicable_state,
            outcome,
        }: FinishedServingStartup,
    ) {
        if let ReplacementOutcome::Failed { error, .. } = &outcome {
            error!("Failed to serve the reconciled state: {error}");
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

    fn prepare(&mut self, agent_applicable_state: AgentApplicableState, retry_delay: RetryDelay) {
        let cancellation_token = self.agent_shutdown.child_token();
        let decisions_to_prepare = self
            .model_serving
            .decisions_to_prepare(&agent_applicable_state);

        self.serving_startup = ServingStartup::Preparing(StartupPhase {
            cancellation_token: cancellation_token.clone(),
            outcome: Box::pin(
                ServingPreparation {
                    agent_applicable_state,
                    decisions_to_prepare,
                    retry_delay,
                }
                .prepare(cancellation_token, self.inference_runtime_context.clone()),
            ),
        });
    }

    fn replace(&mut self, prepared_serving: PreparedServing) {
        if let ModelServing::Serving(served_model) = &self.model_serving
            && let AgentApplicableState::WithModel(desired_settings) =
                &prepared_serving.agent_applicable_state
            && served_model.model_serving_settings == *desired_settings
        {
            self.resume_serving(served_model, prepared_serving.agent_applicable_state);

            return;
        }

        let held = self.withdraw_serving();
        let cancellation_token = self.agent_shutdown.child_token();

        self.serving_startup = ServingStartup::Replacing(StartupPhase {
            cancellation_token: cancellation_token.clone(),
            outcome: Box::pin(
                ServingReplacement {
                    held,
                    prepared_serving,
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
                self.prepare(agent_applicable_state, RetryDelay::AfterRetryInterval);
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
        self.prepare(agent_applicable_state, RetryDelay::Immediately);
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
