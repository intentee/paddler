use crate::request_params_validation_error::RequestParamsValidationError;

pub trait Validates<TOutput> {
    fn validate(self) -> Result<TOutput, RequestParamsValidationError>;
}
