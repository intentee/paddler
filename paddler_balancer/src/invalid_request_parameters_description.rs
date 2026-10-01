use paddler_messaging::request_params_validation_error::RequestParamsValidationError;

#[must_use]
pub fn invalid_request_parameters_description(
    validation_error: &RequestParamsValidationError,
) -> String {
    format!("Invalid request parameters: {validation_error}")
}
