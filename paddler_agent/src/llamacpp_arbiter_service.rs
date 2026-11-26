use std::convert::Infallible;
use std::sync::Arc;

use anyhow::Result;
use tokio::select;
use tokio_util::sync::CancellationToken;
use trzcina::service::Service;

use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_outbox::outbox::Outbox;

use crate::agent_applicable_state_holder::AgentApplicableStateHolder;
use crate::arbitrated_state::ArbitratedState;
use crate::pipeline_arbiter::PipelineArbiter;
use crate::pipeline_arbiter_params::PipelineArbiterParams;
use crate::pipeline_request::PipelineRequest;
use crate::reconciled_state::ReconciledState;
use crate::reconciled_state_holder::ReconciledStateHolder;

pub struct LlamaCppArbiterService {
    pub inference_runtime_context: InferenceRuntimeContext,
    pub pipeline_requests: Arc<Outbox<PipelineRequest>>,
    pub reconciled_state_holder: Arc<ReconciledStateHolder>,
    pub served_applicable_state_holder: Arc<AgentApplicableStateHolder>,
}

impl LlamaCppArbiterService {
    async fn arbitrate_pipelines_until_shutdown(self: Box<Self>, shutdown: CancellationToken) {
        let Self {
            inference_runtime_context,
            pipeline_requests,
            reconciled_state_holder,
            served_applicable_state_holder,
        } = *self;
        let slot_aggregated_status = inference_runtime_context.slot_aggregated_status.clone();
        let mut reconciled_state_rx = reconciled_state_holder.subscribe();
        let mut arbitrated_state = ArbitratedState::NothingYet;
        let mut pipeline_arbiter = PipelineArbiter::new(PipelineArbiterParams {
            agent_shutdown: shutdown.clone(),
            inference_runtime_context,
            served_applicable_state_holder,
        });

        loop {
            select! {
                biased;
                () = shutdown.cancelled() => break,
                Ok(()) = reconciled_state_rx.changed() => {
                    let ReconciledState {
                        agent_applicable_state,
                        desired_state_version,
                    } = reconciled_state_rx.borrow_and_update().clone();

                    if !arbitrated_state.arbitrates(&agent_applicable_state) {
                        pipeline_arbiter.apply(agent_applicable_state.clone());
                        arbitrated_state = ArbitratedState::Arbitrating(agent_applicable_state);
                    }

                    slot_aggregated_status.acknowledge_desired_state_version(desired_state_version);
                }
                finished_serving_startup = pipeline_arbiter.next_finished_startup() => {
                    pipeline_arbiter.conclude(finished_serving_startup);
                }
                pipeline_request = pipeline_requests.next() => pipeline_arbiter.forward(pipeline_request),
            }
        }

        pipeline_arbiter.shut_down().await;
    }
}

impl Service for LlamaCppArbiterService {
    type Error = Infallible;

    fn name(&self) -> &'static str {
        "agent::llamacpp_arbiter_service"
    }

    async fn run(self: Box<Self>, shutdown: CancellationToken) -> Result<(), Infallible> {
        self.arbitrate_pipelines_until_shutdown(shutdown).await;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tokio_util::sync::CancellationToken;
    use trzcina::service::Service;

    use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
    use paddler_agent_runtime::inference_runtime_context_params::InferenceRuntimeContextParams;
    use paddler_outbox::outbox::Outbox;

    use super::LlamaCppArbiterService;
    use crate::agent_applicable_state_holder::AgentApplicableStateHolder;
    use crate::reconciled_state_holder::ReconciledStateHolder;

    #[tokio::test(flavor = "multi_thread")]
    async fn exits_when_the_agent_shuts_down() {
        let agent_shutdown = CancellationToken::new();
        let service = LlamaCppArbiterService {
            inference_runtime_context: InferenceRuntimeContext::new(
                InferenceRuntimeContextParams {
                    agent_name: None,
                    slots: 1,
                },
            ),
            pipeline_requests: Arc::new(Outbox::default()),
            reconciled_state_holder: Arc::new(ReconciledStateHolder::default()),
            served_applicable_state_holder: Arc::new(AgentApplicableStateHolder::default()),
        };

        agent_shutdown.cancel();

        Box::new(service)
            .run(agent_shutdown)
            .await
            .expect("the service must finish cleanly once the agent shuts down");
    }
}
