#[derive(Debug, thiserror::Error)]
pub enum OpenAIConformanceError {
    #[error(
        "chat completion request does not conform to the official OpenAI schema: {violations:?}"
    )]
    RequestDoesNotConform { violations: Vec<String> },
    #[error(
        "chat completion response does not conform to the official OpenAI schema: {violations:?}"
    )]
    ResponseDoesNotConform { violations: Vec<String> },
    #[error(
        "chat completion stream chunk does not conform to the official OpenAI schema: {violations:?}"
    )]
    StreamChunkDoesNotConform { violations: Vec<String> },
}
