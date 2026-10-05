use paddler_messaging::inference_client::message::Message as OutgoingMessage;

use crate::chunk_forwarding_session_controller::transform_result::TransformResult;
use crate::compatibility::openai_service::openai_error::OpenAIError;

#[must_use]
pub fn try_universal_error_chunk(message: &OutgoingMessage) -> Option<TransformResult> {
    OpenAIError::classify(message)
        .map(|error| TransformResult::Error(error.to_envelope().to_string()))
}
