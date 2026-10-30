use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_embeddings::embedding_error::EmbeddingError;
use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("the decision pipeline failed: {0}")]
    DecisionPipeline(#[from] DecisionError),

    #[error("the embedding pipeline failed: {0}")]
    EmbeddingPipeline(#[from] EmbeddingError),

    #[error("the model failed to load: {0}")]
    ModelLoad(#[from] AgentRuntimeError),

    #[error("the text generation pipeline failed: {0}")]
    TextGenerationPipeline(#[from] TextGenerationError),
}
