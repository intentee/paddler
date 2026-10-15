use thiserror::Error;

#[derive(Debug, Eq, Error, PartialEq)]
pub enum RequestParamsValidationError {
    #[error("question '{question_id}' appears more than once")]
    DecisionQuestionIdRepeated { question_id: String },
    #[error("question '{question_id}' has no options")]
    DecisionQuestionWithoutOptions { question_id: String },
    #[error("a decision needs at least one question")]
    DecisionWithoutQuestions,
    #[error("Required field '{field}' not found in properties")]
    RequiredFieldNotInProperties { field: String },
    #[error("parse_tool_calls requires at least one tool")]
    ToolCallParsingWithoutTools,
}
