use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::inference_client::message::Message as OutgoingMessage;
use paddler_messaging::inference_client::response::Response as OutgoingResponse;
use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
use paddler_messaging::jsonrpc::error_envelope::ErrorEnvelope;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_messaging::oversized_image_details::OversizedImageDetails;
use paddler_messaging::raw_tool_call_tokens::RawToolCallTokens;
use serde_json::Value;
use serde_json::json;

fn validation_failure_message(errors: &[String]) -> String {
    errors
        .first()
        .cloned()
        .unwrap_or_else(|| "tool call failed validation".to_owned())
}

fn unrecognized_tool_call_format_message(raw: &RawToolCallTokens) -> String {
    format!(
        "model produced output the parser did not recognise as any registered tool-call format; \
         FFI error: {}; raw text: {}",
        raw.ffi_error_message, raw.text,
    )
}

fn image_exceeds_batch_size_message(details: &OversizedImageDetails) -> String {
    format!(
        "image required {} tokens but agent n_batch is {}; rerun with a larger n_batch",
        details.image_tokens, details.n_batch,
    )
}

fn description_from_error_token(token: &GeneratedTokenResult) -> Option<&str> {
    match token {
        GeneratedTokenResult::ChatTemplateError(description)
        | GeneratedTokenResult::GrammarIncompatibleWithThinking(description)
        | GeneratedTokenResult::GrammarRejectedModelOutput(description)
        | GeneratedTokenResult::GrammarInitializationFailed(description)
        | GeneratedTokenResult::GrammarSyntaxError(description)
        | GeneratedTokenResult::ImageDecodingFailed(description)
        | GeneratedTokenResult::MultimodalNotSupported(description)
        | GeneratedTokenResult::SamplerError(description)
        | GeneratedTokenResult::ToolCallParseFailed(description)
        | GeneratedTokenResult::ToolSchemaInvalid(description) => Some(description),
        _ => None,
    }
}

fn server_error_from_token(token: &GeneratedTokenResult) -> Option<OpenAIError> {
    match token {
        GeneratedTokenResult::ImageExceedsBatchSize(details) => Some(OpenAIError {
            error_type: "server_error",
            message: image_exceeds_batch_size_message(details),
        }),
        GeneratedTokenResult::ToolCallValidationFailed(errors) => Some(OpenAIError {
            error_type: "server_error",
            message: validation_failure_message(errors),
        }),
        GeneratedTokenResult::UnrecognizedToolCallFormat(raw) => Some(OpenAIError {
            error_type: "server_error",
            message: unrecognized_tool_call_format_message(raw),
        }),
        other => description_from_error_token(other).map(|description| OpenAIError {
            error_type: "server_error",
            message: description.to_owned(),
        }),
    }
}

pub struct OpenAIError {
    pub error_type: &'static str,
    pub message: String,
}

impl OpenAIError {
    #[must_use]
    pub fn classify(message: &OutgoingMessage) -> Option<Self> {
        match message {
            OutgoingMessage::Error(ErrorEnvelope {
                error: JsonRpcError { description, .. },
                ..
            }) => Some(Self {
                error_type: "server_error",
                message: description.clone(),
            }),
            OutgoingMessage::Response(ResponseEnvelope { response, .. }) => match response {
                OutgoingResponse::GeneratedToken(token) => server_error_from_token(token),
                OutgoingResponse::Timeout => Some(Self {
                    error_type: "timeout",
                    message: "request timed out".to_owned(),
                }),
                OutgoingResponse::TooManyBufferedRequests => Some(Self {
                    error_type: "rate_limit_error",
                    message: "too many buffered requests".to_owned(),
                }),
                OutgoingResponse::Embedding(_) => None,
            },
        }
    }

    #[must_use]
    pub fn to_envelope(&self) -> Value {
        json!({
            "error": {
                "message": self.message,
                "type": self.error_type,
                "param": null,
                "code": null
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings_types::ToolCallArguments;
    use paddler_messaging::embedding_result::EmbeddingResult;
    use paddler_messaging::generation_summary::GenerationSummary;

    use super::OpenAIError;
    use super::OutgoingMessage;
    use super::OutgoingResponse;
    use super::ResponseEnvelope;
    use super::validation_failure_message;
    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
    use paddler_messaging::jsonrpc::error_envelope::ErrorEnvelope;

    fn token_message(token_result: GeneratedTokenResult) -> OutgoingMessage {
        OutgoingMessage::Response(ResponseEnvelope {
            generated_by: None,
            request_id: "test-request".to_owned(),
            response: OutgoingResponse::GeneratedToken(token_result),
        })
    }

    #[test]
    fn to_envelope_has_the_openai_error_shape() {
        let envelope = OpenAIError {
            error_type: "server_error",
            message: "something went wrong".to_owned(),
        }
        .to_envelope();

        assert_eq!(envelope["error"]["type"], "server_error");
        assert_eq!(envelope["error"]["message"], "something went wrong");
        assert!(envelope["error"]["param"].is_null());
        assert!(envelope["error"]["code"].is_null());
    }

    #[test]
    fn validation_failure_message_returns_first_error() {
        let message =
            validation_failure_message(&["first issue".to_owned(), "second issue".to_owned()]);

        assert_eq!(message, "first issue");
    }

    #[test]
    fn validation_failure_message_falls_back_when_no_errors() {
        let message = validation_failure_message(&[]);

        assert!(message.contains("validation"));
    }

    #[test]
    fn classifies_jsonrpc_error_as_server_error() {
        let message = OutgoingMessage::Error(ErrorEnvelope {
            request_id: "test-request".to_owned(),
            error: JsonRpcError {
                code: 500,
                description: "internal failure".to_owned(),
            },
        });

        let classified = OpenAIError::classify(&message).unwrap();

        assert_eq!(classified.error_type, "server_error");
        assert_eq!(classified.message, "internal failure");
    }

    #[test]
    fn classifies_timeout_as_timeout() {
        let message = OutgoingMessage::Response(ResponseEnvelope {
            generated_by: None,
            request_id: "test-request".to_owned(),
            response: OutgoingResponse::Timeout,
        });

        let classified = OpenAIError::classify(&message).unwrap();

        assert_eq!(classified.error_type, "timeout");
    }

    #[test]
    fn classifies_too_many_buffered_requests_as_rate_limit() {
        let message = OutgoingMessage::Response(ResponseEnvelope {
            generated_by: None,
            request_id: "test-request".to_owned(),
            response: OutgoingResponse::TooManyBufferedRequests,
        });

        let classified = OpenAIError::classify(&message).unwrap();

        assert_eq!(classified.error_type, "rate_limit_error");
    }

    #[test]
    fn classifies_validation_failure_with_first_message() {
        let classified = OpenAIError::classify(&token_message(
            GeneratedTokenResult::ToolCallValidationFailed(vec!["missing field x".to_owned()]),
        ))
        .unwrap();

        assert_eq!(classified.error_type, "server_error");
        assert_eq!(classified.message, "missing field x");
    }

    #[test]
    fn does_not_classify_a_content_token() {
        assert!(
            OpenAIError::classify(&token_message(GeneratedTokenResult::ContentToken(
                "hello".to_owned()
            )))
            .is_none()
        );
    }

    #[test]
    fn does_not_classify_a_done_summary() {
        assert!(
            OpenAIError::classify(&token_message(GeneratedTokenResult::Done(
                GenerationSummary::default()
            )))
            .is_none()
        );
    }

    #[test]
    fn does_not_classify_an_embedding_response() {
        let message = OutgoingMessage::Response(ResponseEnvelope {
            generated_by: None,
            request_id: "test-request".to_owned(),
            response: OutgoingResponse::Embedding(EmbeddingResult::Done),
        });

        assert!(OpenAIError::classify(&message).is_none());
    }

    #[test]
    fn classifies_a_tool_call_with_arguments_is_unrelated_to_errors() {
        let parsed = vec![llama_cpp_bindings_types::ParsedToolCall::new(
            "call_x".to_owned(),
            "get_weather".to_owned(),
            ToolCallArguments::ValidJson(serde_json::json!({"location": "Paris"})),
        )];

        assert!(
            OpenAIError::classify(&token_message(GeneratedTokenResult::ToolCallParsed(parsed)))
                .is_none()
        );
    }
}
