/// One specific reason a parsed tool call failed validation.
///
/// The variants split per failure mode rather than collapsing into a generic
/// `Other(String)` so callers can decide whether to surface the failure to
/// the model (retry-on-validation-failure) or to the client.
#[derive(Debug, thiserror::Error)]
pub enum ToolCallValidationError {
    #[error("unknown tool name {0:?}")]
    UnknownToolName(String),
    #[error("arguments for tool {tool_name:?} are not valid JSON: {message}")]
    InvalidJson {
        tool_name: String,
        message: String,
    },
    #[error("arguments for tool {tool_name:?} must be a JSON object, got {kind}")]
    NotAnObject {
        tool_name: String,
        /// JSON value kind word (`array`, `string`, `number`, etc.).
        kind: &'static str,
    },
    #[error("arguments for tool {tool_name:?} failed schema check: {message}")]
    SchemaMismatch {
        tool_name: String,
        message: String,
    },
}
