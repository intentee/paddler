use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tokio::select;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use trzcina::Service;

use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;

use crate::agent_applicable_state_holder::AgentApplicableStateHolder;
use crate::pipeline_arbiter::PipelineArbiter;
use crate::pipeline_arbiter_params::PipelineArbiterParams;
use crate::pipeline_request::PipelineRequest;

pub struct LlamaCppArbiterService {
    pub agent_applicable_state_holder: Arc<AgentApplicableStateHolder>,
    pub inference_runtime_context: InferenceRuntimeContext,
    pub pipeline_request_rx: mpsc::UnboundedReceiver<PipelineRequest>,
    pub served_applicable_state_holder: Arc<AgentApplicableStateHolder>,
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
                pipeline_request = pipeline_request_rx.recv() => match pipeline_request {
                    Some(pipeline_request) => pipeline_arbiter.forward(pipeline_request),
                    None => break Ok(()),
                },
            }
        };

        pipeline_arbiter.shut_down().await;

        service_outcome
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
    use paddler_agent_runtime::shared_llama_backend::SharedLlamaBackend;
    use paddler_agent_runtime::system_parallelism::SystemParallelism;
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
                available_parallelism: Arc::new(SystemParallelism),
                llama_backend: Arc::new(SharedLlamaBackend::default()),
                model_metadata_holder: Arc::new(ModelMetadataHolder::default()),
                slot_aggregated_status: Arc::new(SlotAggregatedStatus::new(1)),
            },
            pipeline_request_rx,
            served_applicable_state_holder: Arc::new(AgentApplicableStateHolder::default()),
        };

        drop(pipeline_request_tx);

        Box::new(service)
            .run(CancellationToken::new())
            .await
            .expect("the service must finish cleanly once no request can arrive");
    }
}
