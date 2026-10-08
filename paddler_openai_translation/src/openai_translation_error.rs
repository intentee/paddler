use std::time::SystemTimeError;

use paddler_messaging::request_params_validation_error::RequestParamsValidationError;

#[derive(Debug, thiserror::Error)]
pub enum OpenAITranslationError {
    #[error("the system clock reads a time before the Unix epoch")]
    ClockBeforeUnixEpoch(#[source] SystemTimeError),

    #[error(transparent)]
    ToolRejected(#[from] RequestParamsValidationError),
}
