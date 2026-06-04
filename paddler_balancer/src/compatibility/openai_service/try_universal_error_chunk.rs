use paddler_messaging::inference_client::message::Message as OutgoingMessage;
use paddler_messaging::inference_client::response::Response as OutgoingResponse;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;

use crate::chunk_forwarding_session_controller::transform_result::TransformResult;
use crate::compatibility::openai_service::openai_error::OpenAIError;

#[must_use]
pub fn try_universal_error_chunk(message: &OutgoingMessage) -> Option<TransformResult> {
    if let OutgoingMessage::Response(ResponseEnvelope {
        response: OutgoingResponse::Embedding(_),
        ..
    }) = message
    {
        return Some(TransformResult::Error(
            OpenAIError {
                error_type: "invalid_request_error",
                message: "unexpected embedding response in chat completions".to_owned(),
            }
            .to_envelope()
            .to_string(),
        ));
    }

    OpenAIError::classify(message)
        .map(|error| TransformResult::Error(error.to_envelope().to_string()))
}
