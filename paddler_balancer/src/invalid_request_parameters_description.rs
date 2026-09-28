#[must_use]
pub fn invalid_request_parameters_description(validation_error: &anyhow::Error) -> String {
    format!("Invalid request parameters: {validation_error}")
}
