use jsonschema::ValidationError;
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ToolCallValidationError {
    #[error("arguments for tool {tool_name:?} are not a JSON object: {arguments}")]
    ArgumentsAreNotAnObject { tool_name: String, arguments: Value },
    #[error("arguments for tool {tool_name:?} are not valid JSON: {raw_arguments:?}")]
    ArgumentsAreNotJson {
        tool_name: String,
        raw_arguments: String,
    },
    #[error("tool {tool_name:?} parameters are not a valid JSON Schema: {source}")]
    InvalidSchema {
        tool_name: String,
        #[source]
        source: Box<ValidationError<'static>>,
    },
    #[error("arguments for tool {tool_name:?} failed schema check: {violations:?}")]
    SchemaMismatch {
        tool_name: String,
        violations: Vec<String>,
    },
    #[error("unknown tool name {0:?}")]
    UnknownToolName(String),
}
