#[derive(Debug, thiserror::Error)]
pub enum ValidatorBuildError {
    #[error("tool {tool_name:?} parameters are not a valid JSON Schema: {message}")]
    InvalidSchema { tool_name: String, message: String },
}
