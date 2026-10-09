use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_decision::decision_request_preparer::DecisionRequestPreparer;
use paddler_agent_embeddings::embedding_batch_preparer::EmbeddingBatchPreparer;
use paddler_agent_embeddings::embedding_error::EmbeddingError;
use paddler_agent_runtime::scheduler_handle::SchedulerHandle;
use paddler_agent_text_generation::generation_request_preparer::GenerationRequestPreparer;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_agent_text_generation::text_generation_request::TextGenerationRequest;
use paddler_messaging::inference_mode::InferenceMode;

use crate::agent_error::AgentError;
use crate::pipeline_request::PipelineRequest;

pub enum RunningPipeline {
    Decision(SchedulerHandle<DecisionRequestPreparer, DecisionError>),
    Embeddings(SchedulerHandle<EmbeddingBatchPreparer, EmbeddingError>),
    TextGeneration(SchedulerHandle<GenerationRequestPreparer, TextGenerationError>),
}

impl RunningPipeline {
    pub fn forward(&self, agent_name: Option<&str>, request: PipelineRequest) {
        match (self, request) {
            (Self::Decision(scheduler_handle), PipelineRequest::Decide(request)) => {
                scheduler_handle.request_preparer.prepare(request);
            }
            (
                Self::Embeddings(scheduler_handle),
                PipelineRequest::GenerateEmbeddingBatch(request),
            ) => {
                scheduler_handle.request_preparer.prepare(request);
            }
            (
                Self::TextGeneration(scheduler_handle),
                PipelineRequest::ContinueFromConversationHistory(request),
            ) => {
                scheduler_handle.request_preparer.prepare(
                    TextGenerationRequest::ContinueFromConversationHistory(request),
                );
            }
            (
                Self::TextGeneration(scheduler_handle),
                PipelineRequest::ContinueFromRawPrompt(request),
            ) => {
                scheduler_handle
                    .request_preparer
                    .prepare(TextGenerationRequest::ContinueFromRawPrompt(request));
            }
            (running_pipeline, request) => request.reject_because_another_mode_is_served(
                agent_name,
                running_pipeline.inference_mode(),
            ),
        }
    }

    pub async fn shut_down(self) -> Result<(), AgentError> {
        match self {
            Self::Decision(scheduler_handle) => Ok(scheduler_handle.shut_down().await?),
            Self::Embeddings(scheduler_handle) => Ok(scheduler_handle.shut_down().await?),
            Self::TextGeneration(scheduler_handle) => Ok(scheduler_handle.shut_down().await?),
        }
    }

    const fn inference_mode(&self) -> InferenceMode {
        match self {
            Self::Decision(_) => InferenceMode::Decision,
            Self::Embeddings(_) => InferenceMode::Embeddings,
            Self::TextGeneration(_) => InferenceMode::TextGeneration,
        }
    }
}
