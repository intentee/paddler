use std::time::SystemTimeError;

use paddler_messaging::inference_mode::InferenceMode;
use paddler_messaging::request_params_validation_error::RequestParamsValidationError;

#[derive(Debug, thiserror::Error)]
pub enum OpenAITranslationError {
    #[error("the system clock reads a time before the Unix epoch")]
    ClockBeforeUnixEpoch(#[source] SystemTimeError),

    #[error("the cluster serves {inference_mode:?}, not text generation")]
    InferenceModeMismatch { inference_mode: InferenceMode },

    #[error(transparent)]
    ToolRejected(#[from] RequestParamsValidationError),
}
