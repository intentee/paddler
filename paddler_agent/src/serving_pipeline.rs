use tokio_util::task::TaskTracker;

use crate::pipeline_request::PipelineRequest;
use crate::running_pipeline::RunningPipeline;

pub enum ServingPipeline {
    Idle,
    Running(RunningPipeline),
}

impl ServingPipeline {
    pub fn forward(&self, agent_name: Option<&str>, request: PipelineRequest) {
        match self {
            Self::Idle => request.reject_because_no_model_is_loaded(agent_name),
            Self::Running(running_pipeline) => running_pipeline.forward(agent_name, request),
        }
    }

    pub async fn shut_down(self) {
        match self {
            Self::Idle => {}
            Self::Running(running_pipeline) => running_pipeline.shut_down().await,
        }
    }

    pub fn shut_down_in_background(self, task_tracker: &TaskTracker) {
        task_tracker.spawn(self.shut_down());
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;
    use std::sync::Arc;

    use tokio::sync::mpsc;

    use paddler_agent_runtime::agent_request::AgentRequest;
    use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
    use paddler_agent_runtime::model_metadata_holder::ModelMetadataHolder;
    use paddler_agent_runtime::shared_llama_backend::SharedLlamaBackend;
    use paddler_agent_runtime::system_parallelism::SystemParallelism;
    use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
    use paddler_agent_status::slot_guard::SlotGuard;
    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;

    use super::ServingPipeline;
    use crate::pipeline_request::PipelineRequest;

    fn inference_runtime_context() -> InferenceRuntimeContext {
        InferenceRuntimeContext {
            agent_name: Some("agent".to_owned()),
            available_parallelism: Arc::new(SystemParallelism),
            llama_backend: Arc::new(SharedLlamaBackend::default()),
            model_metadata_holder: Arc::new(ModelMetadataHolder::default()),
            slot_aggregated_status: Arc::new(SlotAggregatedStatus::new(1)),
        }
    }

    #[tokio::test]
    async fn an_idle_serving_pipeline_rejects_a_request_because_no_model_is_loaded() {
        let inference_runtime_context = inference_runtime_context();
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();
        let (_generate_tokens_stop_tx, generate_tokens_stop_rx) = mpsc::unbounded_channel();

        ServingPipeline::Idle.forward(
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
}
