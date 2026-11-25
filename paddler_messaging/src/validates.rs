use crate::request_params_validation_error::RequestParamsValidationError;

pub trait Validates<TOutput> {
    /// # Errors
    ///
    /// Returns [`RequestParamsValidationError::DecisionWithoutQuestions`], [`RequestParamsValidationError::DecisionQuestionIdRepeated`] or [`RequestParamsValidationError::DecisionQuestionWithoutOptions`] when decision questions are missing, repeated or without options, [`RequestParamsValidationError::ToolCallParsingWithoutTools`] when tool call parsing is requested without tools, and [`RequestParamsValidationError::RequiredFieldNotInProperties`] when a tool parameters schema requires a field it does not define.
    fn validate(self) -> Result<TOutput, RequestParamsValidationError>;
}
