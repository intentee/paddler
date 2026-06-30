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

#[cfg(test)]
mod tests {
    use paddler_messaging::embedding_result::EmbeddingResult;
    use paddler_messaging::inference_client::message::Message as OutgoingMessage;
    use paddler_messaging::inference_client::response::Response as OutgoingResponse;
    use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
    use paddler_messaging::jsonrpc::error_envelope::ErrorEnvelope;
    use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;

    use super::responses_error;

    #[test]
    fn embedding_response_is_an_invalid_request_error() {
        let message = OutgoingMessage::Response(ResponseEnvelope {
            generated_by: None,
            request_id: "test-request".to_owned(),
            response: OutgoingResponse::Embedding(EmbeddingResult::Done),
        });

        let error =
            responses_error(&message).expect("an embedding response inside /responses is an error");

        assert_eq!(error.error_type, "invalid_request_error");
    }

    #[test]
    fn non_embedding_message_delegates_to_openai_error_classify() {
        let message = OutgoingMessage::Error(ErrorEnvelope {
            request_id: "test-request".to_owned(),
            error: JsonRpcError {
                code: 500,
                description: "internal failure".to_owned(),
            },
        });

        let error = responses_error(&message).expect("a jsonrpc error classifies as an error");

        assert_eq!(error.error_type, "server_error");
    }
}
