use anyhow::Context as _;
use anyhow::Result;
use log::error;
use log::info;
use log::warn;
use tokio_util::sync::CancellationToken;

use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;

use crate::agent_applicable_model::AgentApplicableModel;
use crate::agent_applicable_state::AgentApplicableState;
use crate::continuous_batch_arbiter::ContinuousBatchArbiter;
use crate::continuous_batch_arbiter_context::ContinuousBatchArbiterContext;
use crate::continuous_batch_arbiter_handle::ContinuousBatchArbiterHandle;
use crate::continuous_batch_arbiter_spawn_outcome::ContinuousBatchArbiterSpawnOutcome;
use crate::continuous_batch_preparation_request::ContinuousBatchPreparationRequest;

pub enum ContinuousBatchArbiterState {
    Idle,
    Running(ContinuousBatchArbiterHandle),
}

impl ContinuousBatchArbiterState {
    pub async fn apply(
        self,
        shutdown: &CancellationToken,
        agent_applicable_state: AgentApplicableState,
        arbiter_context: &ContinuousBatchArbiterContext,
    ) -> Self {
        match self
            .try_to_apply(shutdown, agent_applicable_state, arbiter_context)
            .await
        {
            Ok(next_state) => next_state,
            Err(application_error) => {
                error!("Failed to apply reconciled state change: {application_error}");

                Self::Idle
            }
        }
    }

    pub fn forward(&self, agent_name: Option<&str>, request: ContinuousBatchPreparationRequest) {
        match self {
            Self::Idle => request.reject_because_no_model_is_loaded(agent_name),
            Self::Running(arbiter_handle) => arbiter_handle.request_preparer.prepare(request),
        }
    }

    pub async fn shut_down(self) -> Result<()> {
        match self {
            Self::Idle => Ok(()),
            Self::Running(arbiter_handle) => arbiter_handle
                .shutdown()
                .await
                .context("Arbiter shutdown returned an error"),
        }
    }

    async fn try_to_apply(
        self,
        shutdown: &CancellationToken,
        agent_applicable_state: AgentApplicableState,
        arbiter_context: &ContinuousBatchArbiterContext,
    ) -> Result<Self> {
        arbiter_context.slot_aggregated_status.reset();
        self.shut_down().await?;

        let next_state = match agent_applicable_state {
            AgentApplicableState {
                chat_template_override,
                inference_parameters,
                model:
                    AgentApplicableModel::Resolved {
                        model_path,
                        multimodal_projection_path,
                    },
            } => {
                Self::spawn(
                    shutdown,
                    ContinuousBatchArbiter {
                        chat_template_override,
                        context: arbiter_context.clone(),
                        inference_parameters,
                        model_path,
                        multimodal_projection_path,
                    },
                )
                .await?
            }
            AgentApplicableState {
                model: AgentApplicableModel::NotConfigured,
                ..
            } => {
                warn!("No model configured in applicable state; skipping llama.cpp initialization");

                Self::Idle
            }
        };

        arbiter_context
            .slot_aggregated_status
            .set_state_application_status(AgentStateApplicationStatus::Applied);

        Ok(next_state)
    }

    async fn spawn(shutdown: &CancellationToken, arbiter: ContinuousBatchArbiter) -> Result<Self> {
        match arbiter.spawn(shutdown).await? {
            ContinuousBatchArbiterSpawnOutcome::Ready(arbiter_handle) => {
                info!("Reconciled state change applied successfully");

                Ok(Self::Running(arbiter_handle))
            }
            ContinuousBatchArbiterSpawnOutcome::Cancelled => {
                info!("Model load was cancelled by shutdown before it finished");

                Ok(Self::Idle)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;
    use std::num::NonZeroU32;
    use std::sync::Arc;

    use tokio::sync::mpsc;
    use tokio_util::sync::CancellationToken;

    use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
    use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;

    use super::ContinuousBatchArbiterState;
    use crate::agent_applicable_state::AgentApplicableState;
    use crate::agent_request::AgentRequest;
    use crate::continuous_batch_arbiter_context::ContinuousBatchArbiterContext;
    use crate::continuous_batch_preparation_request::ContinuousBatchPreparationRequest;
    use crate::model_metadata_holder::ModelMetadataHolder;
    use crate::slot_guard::SlotGuard;

    fn arbiter_context() -> ContinuousBatchArbiterContext {
        ContinuousBatchArbiterContext {
            agent_name: Some("agent".to_owned()),
            model_metadata_holder: Arc::new(ModelMetadataHolder::default()),
            slot_aggregated_status: Arc::new(SlotAggregatedStatus::new(1)),
        }
    }

    async fn applied(
        agent_applicable_state: AgentApplicableState,
        arbiter_context: &ContinuousBatchArbiterContext,
    ) -> ContinuousBatchArbiterState {
        ContinuousBatchArbiterState::Idle
            .apply(
                &CancellationToken::new(),
                agent_applicable_state,
                arbiter_context,
            )
            .await
    }

    #[tokio::test]
    async fn an_idle_arbiter_rejects_a_request_because_no_model_is_loaded() {
        let arbiter_context = arbiter_context();
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();
        let (_generate_tokens_stop_tx, generate_tokens_stop_rx) = mpsc::unbounded_channel();

        ContinuousBatchArbiterState::Idle.forward(
            arbiter_context.agent_name.as_deref(),
            ContinuousBatchPreparationRequest::ContinueFromRawPrompt(AgentRequest {
                params: ContinueFromRawPromptParams {
                    grammar: None,
                    max_tokens: NonZeroU32::MIN,
                    raw_prompt: "Hello".to_owned(),
                },
                response_tx: generated_tokens_tx,
                slot_guard: SlotGuard::new(arbiter_context.slot_aggregated_status.clone()),
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
            arbiter_context
                .slot_aggregated_status
                .slots_processing_count(),
            0
        );
    }

    #[tokio::test]
    async fn applying_a_state_without_a_model_stays_idle_and_reports_applied() {
        let arbiter_context = arbiter_context();

        let arbiter_state = applied(AgentApplicableState::default(), &arbiter_context).await;

        assert_eq!(
            discriminant(&arbiter_state),
            discriminant(&ContinuousBatchArbiterState::Idle)
        );
        assert_eq!(
            arbiter_context
                .slot_aggregated_status
                .get_state_application_status(),
            AgentStateApplicationStatus::Applied
        );
    }
}
