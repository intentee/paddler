#[derive(Debug, thiserror::Error)]
pub enum OpenAIValidatorError {
    #[error("chat completion request does not conform to the OpenAI schema: {violations:?}")]
    RequestDoesNotConform { violations: Vec<String> },
    #[error("chat completion response does not conform to the OpenAI schema: {violations:?}")]
    ResponseDoesNotConform { violations: Vec<String> },
    #[error("chat completion stream chunk does not conform to the OpenAI schema: {violations:?}")]
    StreamChunkDoesNotConform { violations: Vec<String> },
    #[error("responses request does not conform to the OpenAI schema: {violations:?}")]
    ResponsesRequestDoesNotConform { violations: Vec<String> },
    #[error("responses response does not conform to the OpenAI schema: {violations:?}")]
    ResponsesResponseDoesNotConform { violations: Vec<String> },
    #[error("responses stream event does not conform to the OpenAI schema: {violations:?}")]
    ResponsesStreamEventDoesNotConform { violations: Vec<String> },
    #[error("error response does not conform to the OpenAI schema: {violations:?}")]
    ErrorResponseDoesNotConform { violations: Vec<String> },
}
