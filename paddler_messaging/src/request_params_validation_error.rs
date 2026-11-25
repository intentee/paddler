use thiserror::Error;

#[derive(Debug, Eq, Error, PartialEq)]
pub enum RequestParamsValidationError {
    #[error("question {question_id:?} of the decision contains a NUL byte")]
    DecisionQuestionContainsNulByte { question_id: String },
    #[error("the state of the decision contains a NUL byte")]
    DecisionStateContainsNulByte,
    #[error("a decision needs at least one question")]
    DecisionWithoutQuestions,
    #[error("Required field '{field}' not found in properties")]
    RequiredFieldNotInProperties { field: String },
    #[error("parse_tool_calls requires at least one tool")]
    ToolCallParsingWithoutTools,
}
