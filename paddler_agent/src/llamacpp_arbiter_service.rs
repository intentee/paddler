use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tokio::select;
use tokio_util::sync::CancellationToken;
use trzcina::Service;

use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_outbox::outbox::Outbox;

use crate::agent_applicable_state_holder::AgentApplicableStateHolder;
use crate::pipeline_arbiter::PipelineArbiter;
use crate::pipeline_arbiter_params::PipelineArbiterParams;
use crate::pipeline_request::PipelineRequest;

pub struct LlamaCppArbiterService {
    pub agent_applicable_state_holder: Arc<AgentApplicableStateHolder>,
    pub inference_runtime_context: InferenceRuntimeContext,
    pub pipeline_requests: Arc<Outbox<PipelineRequest>>,
    pub served_applicable_state_holder: Arc<AgentApplicableStateHolder>,
}

impl LlamaCppArbiterService {
    async fn arbitrate_pipelines_until_shutdown(
        self: Box<Self>,
        shutdown: CancellationToken,
    ) -> Result<()> {
        let Self {
            agent_applicable_state_holder,
            inference_runtime_context,
            pipeline_requests,
            served_applicable_state_holder,
        } = *self;
        let mut agent_applicable_state_rx = agent_applicable_state_holder.subscribe();
        let mut pipeline_arbiter = PipelineArbiter::new(PipelineArbiterParams {
            agent_shutdown: shutdown.clone(),
            inference_runtime_context,
            served_applicable_state_holder,
        });

        let service_outcome = loop {
            select! {
                biased;
                () = shutdown.cancelled() => break Ok(()),
                Ok(()) = agent_applicable_state_rx.changed() => {
                    pipeline_arbiter.apply(agent_applicable_state_rx.borrow_and_update().clone());
                }
                finished_pipeline_startup = pipeline_arbiter.next_finished_startup() => {
                    pipeline_arbiter.conclude(finished_pipeline_startup);
                }
                pipeline_request = pipeline_requests.next() => pipeline_arbiter.forward(pipeline_request),
            }
        };

        pipeline_arbiter.shut_down().await;

        service_outcome
    }
}

#[async_trait]
impl Service for LlamaCppArbiterService {
    fn name(&self) -> &'static str {
        "agent::llamacpp_arbiter_service"
    }

    async fn run(self: Box<Self>, shutdown: CancellationToken) -> Result<()> {
        self.arbitrate_pipelines_until_shutdown(shutdown).await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tokio_util::sync::CancellationToken;
    use trzcina::Service;

    use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
    use paddler_agent_runtime::inference_runtime_context_params::InferenceRuntimeContextParams;
    use paddler_outbox::outbox::Outbox;

    use super::LlamaCppArbiterService;
    use crate::agent_applicable_state_holder::AgentApplicableStateHolder;

    #[tokio::test(flavor = "multi_thread")]
    async fn exits_when_the_agent_shuts_down() {
        let agent_shutdown = CancellationToken::new();
        let service = LlamaCppArbiterService {
            agent_applicable_state_holder: Arc::new(AgentApplicableStateHolder::default()),
            inference_runtime_context: InferenceRuntimeContext::new(
                InferenceRuntimeContextParams {
                    agent_name: None,
                    slots: 1,
                },
            ),
            pipeline_requests: Arc::new(Outbox::default()),
            served_applicable_state_holder: Arc::new(AgentApplicableStateHolder::default()),
        };

        agent_shutdown.cancel();

        Box::new(service)
            .run(agent_shutdown)
            .await
            .expect("the service must finish cleanly once the agent shuts down");
    }
}
