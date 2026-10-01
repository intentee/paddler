use thiserror::Error;

#[derive(Debug, Eq, Error, PartialEq)]
pub enum RequestParamsValidationError {
    #[error("Required field '{field}' not found in properties")]
    RequiredFieldNotInProperties { field: String },
    #[error("parse_tool_calls requires at least one tool")]
    ToolCallParsingWithoutTools,
}
