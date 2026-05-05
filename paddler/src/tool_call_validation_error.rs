#[derive(Debug, thiserror::Error)]
pub enum ToolCallValidationError {
    #[error("unknown tool name {0:?}")]
    UnknownToolName(String),
    #[error("arguments for tool {tool_name:?} failed schema check: {message}")]
    SchemaMismatch {
        tool_name: String,
        message: String,
    },
}
