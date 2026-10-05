use paddler_messaging::inference_client::message::Message as OutgoingMessage;
use paddler_messaging::inference_client::response::Response as OutgoingResponse;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;

use crate::compatibility::openai_service::openai_error::OpenAIError;

#[must_use]
pub fn responses_error(message: &OutgoingMessage) -> Option<OpenAIError> {
    if let OutgoingMessage::Response(ResponseEnvelope {
        response: OutgoingResponse::Embedding(_),
        ..
    }) = message
    {
        return Some(OpenAIError {
            error_type: "invalid_request_error",
            message: "unexpected embedding response in responses".to_owned(),
        });
    }

    OpenAIError::classify(message)
}
