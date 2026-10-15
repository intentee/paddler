use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_embeddings::embedding_error::EmbeddingError;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("the decision pipeline failed")]
    DecisionPipeline(#[from] DecisionError),

    #[error("the embedding pipeline failed")]
    EmbeddingPipeline(#[from] EmbeddingError),

    #[error("the text generation pipeline failed")]
    TextGenerationPipeline(#[from] TextGenerationError),
}
