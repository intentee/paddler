#[derive(Debug, thiserror::Error)]
pub enum ValidatorBuildError {
    #[error("could not serialize tool {tool_name:?} parameters to JSON: {message}")]
    SerializationFailed { tool_name: String, message: String },
    #[error("tool {tool_name:?} parameters are not a valid JSON Schema: {message}")]
    InvalidSchema { tool_name: String, message: String },
}
