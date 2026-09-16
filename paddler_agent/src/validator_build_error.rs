#[derive(Debug, thiserror::Error)]
pub enum ValidatorBuildError {
    #[error("serialized tool at index {tool_index} is invalid: {message}")]
    InvalidSerializedTool {
        tool_index: usize,
        message: &'static str,
    },
    #[error("tool {tool_name:?} parameters are not a valid JSON Schema: {message}")]
    InvalidSchema { tool_name: String, message: String },
}
