use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use log::error;
use tokio::select;
use tokio::sync::mpsc;
use tokio::time::Duration;
use tokio::time::Instant;
use tokio::time::MissedTickBehavior;
use tokio::time::interval_at;
use tokio_util::sync::CancellationToken;
use trzcina::Service;

use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;

use crate::agent_applicable_state_holder::AgentApplicableStateHolder;
use crate::pipeline_arbiter_state::PipelineArbiterState;
use crate::pipeline_request::PipelineRequest;

const STATE_APPLICATION_RETRY_INTERVAL: Duration = Duration::from_secs(1);

pub struct LlamaCppArbiterService {
    pub agent_applicable_state_holder: Arc<AgentApplicableStateHolder>,
    pub inference_runtime_context: InferenceRuntimeContext,
    pub pipeline_request_rx: mpsc::UnboundedReceiver<PipelineRequest>,
}

#[async_trait]
impl Service for LlamaCppArbiterService {
    fn name(&self) -> &'static str {
        "agent::llamacpp_arbiter_service"
    }

    async fn run(self: Box<Self>, shutdown: CancellationToken) -> Result<()> {
        let Self {
            agent_applicable_state_holder,
            inference_runtime_context,
            mut pipeline_request_rx,
        } = *self;

        let mut arbiter_state = PipelineArbiterState::Idle;
        let mut reconciled_state = agent_applicable_state_holder.subscribe();
        let mut ticker = interval_at(
            Instant::now() + STATE_APPLICATION_RETRY_INTERVAL,
            STATE_APPLICATION_RETRY_INTERVAL,
        );

        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        let shutdown_outcome = loop {
            select! {
                biased;
                () = shutdown.cancelled() => break Ok(()),
                _ = ticker.tick() => {
                    let slot_aggregated_status = &inference_runtime_context.slot_aggregated_status;
                    let current_status = slot_aggregated_status.get_state_application_status();

                    if current_status.should_try_to_apply() {
                        slot_aggregated_status.set_state_application_status(
                            if matches!(current_status, AgentStateApplicationStatus::AttemptedAndRetrying) {
                                AgentStateApplicationStatus::Stuck
                            } else {
                                AgentStateApplicationStatus::AttemptedAndRetrying
                            }
                        );

                        let agent_applicable_state = reconciled_state.borrow().clone();

                        arbiter_state = arbiter_state
                            .apply(&shutdown, agent_applicable_state, &inference_runtime_context)
                            .await;
                    }
                }
                Ok(()) = reconciled_state.changed() => {
                    let agent_applicable_state = reconciled_state.borrow_and_update().clone();

                    inference_runtime_context
                        .slot_aggregated_status
                        .set_state_application_status(AgentStateApplicationStatus::Fresh);

                    arbiter_state = arbiter_state
                        .apply(&shutdown, agent_applicable_state, &inference_runtime_context)
                        .await;
                }
                request = pipeline_request_rx.recv() => match request {
                    Some(request) => arbiter_state.forward(inference_runtime_context.agent_name.as_deref(), request),
                    None => break Ok(()),
                },
            }
        };

        if let Err(shutdown_error) = arbiter_state.shut_down().await {
            error!("Failed to shut down arbiter cleanly: {shutdown_error:#}");
        }

        shutdown_outcome
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tokio::sync::mpsc;
    use tokio_util::sync::CancellationToken;
    use trzcina::Service;

    use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
    use paddler_agent_runtime::model_metadata_holder::ModelMetadataHolder;
    use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;

    use super::LlamaCppArbiterService;
    use crate::agent_applicable_state_holder::AgentApplicableStateHolder;

    #[tokio::test(flavor = "multi_thread")]
    async fn exits_when_its_request_channel_closes() {
        let (pipeline_request_tx, pipeline_request_rx) = mpsc::unbounded_channel();
        let service = LlamaCppArbiterService {
            agent_applicable_state_holder: Arc::new(AgentApplicableStateHolder::default()),
            inference_runtime_context: InferenceRuntimeContext {
                agent_name: None,
                model_metadata_holder: Arc::new(ModelMetadataHolder::default()),
                slot_aggregated_status: Arc::new(SlotAggregatedStatus::new(1)),
            },
            pipeline_request_rx,
        };

        drop(pipeline_request_tx);

        Box::new(service)
            .run(CancellationToken::new())
            .await
            .expect("the service must finish cleanly once no request can arrive");
    }
}
