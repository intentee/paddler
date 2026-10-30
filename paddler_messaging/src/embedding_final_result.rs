#[derive(Debug, Eq, PartialEq)]
pub enum EmbeddingFinalResult {
    AgentRuntimeFailed(String),
    BatchAssemblyFailed(String),
    DecodeFailed(String),
    Done,
    EmbeddingTooLongForRmsNormalization(String),
    EmbeddingsUnavailable(String),
    InputTokenizationFailed(String),
    ModelNotLoaded(String),
    NoEmbeddingsProduced,
    SchedulerUnavailable(String),
}
