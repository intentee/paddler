use actix_web::HttpResponse;
use actix_web::http::StatusCode;
use serde_json::Value;
use serde_json::json;

use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::inference_client::message::Message as OutgoingMessage;
use paddler_messaging::inference_client::response::Response as OutgoingResponse;
use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
use paddler_messaging::jsonrpc::error_envelope::ErrorEnvelope;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_messaging::oversized_media_details::OversizedMediaDetails;
use paddler_messaging::oversized_prompt_details::OversizedPromptDetails;
use paddler_messaging::raw_tool_call_tokens::RawToolCallTokens;

use crate::compatibility::openai_service::openai_error_type::OpenAIErrorType;

fn validation_failure_message(errors: &[String]) -> String {
    errors.join("; ")
}

fn unrecognized_tool_call_format_message(raw: &RawToolCallTokens) -> String {
    format!(
        "model produced output the parser did not recognise as any registered tool-call format; \
         FFI error: {}; raw text: {}",
        raw.ffi_error_message, raw.text,
    )
}

fn media_exceeds_micro_batch_message(details: &OversizedMediaDetails) -> String {
    format!(
        "media required {} tokens but one agent micro batch holds {} tokens",
        details.media_tokens, details.micro_batch_tokens,
    )
}

fn prompt_exceeds_context_size_message(details: &OversizedPromptDetails) -> String {
    format!(
        "prompt has {} tokens but each agent sequence holds {} tokens; shorten the prompt or raise context_size",
        details.prompt_tokens, details.sequence_context_size,
    )
}

const fn openai_error(error_type: OpenAIErrorType, message: String) -> OpenAIError {
    OpenAIError {
        error_type,
        message,
    }
}

fn server_error_from_token(token: &GeneratedTokenResult) -> Option<OpenAIError> {
    match token {
        GeneratedTokenResult::MediaExceedsMicroBatch(details) => Some(openai_error(
            OpenAIErrorType::InvalidRequestError,
            media_exceeds_micro_batch_message(details),
        )),
        GeneratedTokenResult::PromptExceedsContextSize(details) => Some(openai_error(
            OpenAIErrorType::InvalidRequestError,
            prompt_exceeds_context_size_message(details),
        )),
        GeneratedTokenResult::GrammarIncompatibleWithThinking(description)
        | GeneratedTokenResult::GrammarSyntaxError(description)
        | GeneratedTokenResult::ImageDecodingFailed(description)
        | GeneratedTokenResult::MultimodalNotSupported(description)
        | GeneratedTokenResult::MultimodalTokenizationFailed(description)
        | GeneratedTokenResult::PromptTokenizationFailed(description)
        | GeneratedTokenResult::ToolSchemaInvalid(description) => Some(openai_error(
            OpenAIErrorType::InvalidRequestError,
            description.clone(),
        )),
        GeneratedTokenResult::ToolCallValidationFailed(errors) => Some(openai_error(
            OpenAIErrorType::ServerError,
            validation_failure_message(errors),
        )),
        GeneratedTokenResult::UnrecognizedToolCallFormat(raw) => Some(openai_error(
            OpenAIErrorType::ServerError,
            unrecognized_tool_call_format_message(raw),
        )),
        GeneratedTokenResult::BatchAssemblyFailed(description)
        | GeneratedTokenResult::ChatTemplateError(description)
        | GeneratedTokenResult::DecodeFailed(description)
        | GeneratedTokenResult::DetokenizationFailed(description)
        | GeneratedTokenResult::GrammarInitializationFailed(description)
        | GeneratedTokenResult::GrammarRejectedModelOutput(description)
        | GeneratedTokenResult::KvCacheClearFailed(description)
        | GeneratedTokenResult::MediaMicroBatchCheckFailed(description)
        | GeneratedTokenResult::ModelNotLoaded(description)
        | GeneratedTokenResult::MultimodalIngestionFailed(description)
        | GeneratedTokenResult::NoSequenceSlotAvailable(description)
        | GeneratedTokenResult::SamplerChainCreationFailed(description)
        | GeneratedTokenResult::SamplerError(description)
        | GeneratedTokenResult::SamplingCandidatesExhausted(description)
        | GeneratedTokenResult::SchedulerUnavailable(description)
        | GeneratedTokenResult::SequenceIdOutOfRange(description)
        | GeneratedTokenResult::TokenGenerationDisabled(description)
        | GeneratedTokenResult::ToolCallParseFailed(description)
        | GeneratedTokenResult::ToolsSerializationFailed(description) => Some(openai_error(
            OpenAIErrorType::ServerError,
            description.clone(),
        )),
        GeneratedTokenResult::ContentToken(_)
        | GeneratedTokenResult::Done(_)
        | GeneratedTokenResult::ReasoningToken(_)
        | GeneratedTokenResult::ToolCallParsed(_)
        | GeneratedTokenResult::ToolCallToken(_)
        | GeneratedTokenResult::UndeterminableToken(_) => None,
    }
}

pub struct OpenAIError {
    pub error_type: OpenAIErrorType,
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
                error_type: OpenAIErrorType::ServerError,
                message: description.clone(),
            }),
            OutgoingMessage::Notification(_) => None,
            OutgoingMessage::Response(ResponseEnvelope { response, .. }) => match response {
                OutgoingResponse::GeneratedToken(token) => server_error_from_token(token),
                OutgoingResponse::Embedding(_) => Some(Self {
                    error_type: OpenAIErrorType::InvalidRequestError,
                    message: "unexpected embedding response to a token generation request"
                        .to_owned(),
                }),
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

    #[must_use]
    pub fn to_http_response(&self, status_code: StatusCode) -> HttpResponse {
        HttpResponse::build(status_code).json(self.to_envelope())
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings_types::ParsedToolCall;
    use llama_cpp_bindings_types::TokenUsage;
    use llama_cpp_bindings_types::ToolCallArguments;
    use serde_json::json;

    use paddler_messaging::embedding_result::EmbeddingResult;
    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use paddler_messaging::generation_finish::GenerationFinish;
    use paddler_messaging::generation_summary::GenerationSummary;
    use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
    use paddler_messaging::jsonrpc::error_envelope::ErrorEnvelope;
    use paddler_messaging::oversized_media_details::OversizedMediaDetails;
    use paddler_messaging::oversized_prompt_details::OversizedPromptDetails;

    use super::OpenAIError;
    use super::OutgoingMessage;
    use super::OutgoingResponse;
    use super::ResponseEnvelope;
    use super::validation_failure_message;
    use crate::compatibility::openai_service::openai_error_type::OpenAIErrorType;

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
            error_type: OpenAIErrorType::ServerError,
            message: "something went wrong".to_owned(),
        }
        .to_envelope();

        assert_eq!(envelope["error"]["type"], "server_error");
        assert_eq!(envelope["error"]["message"], "something went wrong");
        assert!(envelope["error"]["param"].is_null());
        assert!(envelope["error"]["code"].is_null());
    }

    #[test]
    fn validation_failure_message_reports_every_error() {
        let message =
            validation_failure_message(&["first issue".to_owned(), "second issue".to_owned()]);

        assert_eq!(message, "first issue; second issue");
    }

    #[test]
    fn classifies_media_exceeding_the_micro_batch_as_invalid_request() {
        let error = OpenAIError::classify(&token_message(
            GeneratedTokenResult::MediaExceedsMicroBatch(OversizedMediaDetails {
                media_tokens: 256,
                micro_batch_tokens: 128,
            }),
        ))
        .unwrap();

        assert_eq!(error.error_type, OpenAIErrorType::InvalidRequestError);
        assert_eq!(
            error.message,
            "media required 256 tokens but one agent micro batch holds 128 tokens"
        );
    }

    #[test]
    fn classifies_client_mistakes_as_invalid_requests() {
        for client_mistake in [
            GeneratedTokenResult::GrammarIncompatibleWithThinking("client mistake".to_owned()),
            GeneratedTokenResult::GrammarSyntaxError("client mistake".to_owned()),
            GeneratedTokenResult::ImageDecodingFailed("client mistake".to_owned()),
            GeneratedTokenResult::MultimodalNotSupported("client mistake".to_owned()),
            GeneratedTokenResult::MultimodalTokenizationFailed("client mistake".to_owned()),
            GeneratedTokenResult::PromptTokenizationFailed("client mistake".to_owned()),
            GeneratedTokenResult::ToolSchemaInvalid("client mistake".to_owned()),
        ] {
            let error = OpenAIError::classify(&token_message(client_mistake)).unwrap();

            assert_eq!(error.error_type, OpenAIErrorType::InvalidRequestError);
            assert_eq!(error.message, "client mistake");
        }
    }

    #[test]
    fn classifies_agent_failures_as_server_errors() {
        for agent_failure in [
            GeneratedTokenResult::BatchAssemblyFailed("agent failure".to_owned()),
            GeneratedTokenResult::ChatTemplateError("agent failure".to_owned()),
            GeneratedTokenResult::DecodeFailed("agent failure".to_owned()),
            GeneratedTokenResult::DetokenizationFailed("agent failure".to_owned()),
            GeneratedTokenResult::GrammarInitializationFailed("agent failure".to_owned()),
            GeneratedTokenResult::GrammarRejectedModelOutput("agent failure".to_owned()),
            GeneratedTokenResult::KvCacheClearFailed("agent failure".to_owned()),
            GeneratedTokenResult::MediaMicroBatchCheckFailed("agent failure".to_owned()),
            GeneratedTokenResult::ModelNotLoaded("agent failure".to_owned()),
            GeneratedTokenResult::MultimodalIngestionFailed("agent failure".to_owned()),
            GeneratedTokenResult::NoSequenceSlotAvailable("agent failure".to_owned()),
            GeneratedTokenResult::SamplerChainCreationFailed("agent failure".to_owned()),
            GeneratedTokenResult::SamplerError("agent failure".to_owned()),
            GeneratedTokenResult::SamplingCandidatesExhausted("agent failure".to_owned()),
            GeneratedTokenResult::SchedulerUnavailable("agent failure".to_owned()),
            GeneratedTokenResult::SequenceIdOutOfRange("agent failure".to_owned()),
            GeneratedTokenResult::TokenGenerationDisabled("agent failure".to_owned()),
            GeneratedTokenResult::ToolCallParseFailed("agent failure".to_owned()),
            GeneratedTokenResult::ToolsSerializationFailed("agent failure".to_owned()),
        ] {
            let error = OpenAIError::classify(&token_message(agent_failure)).unwrap();

            assert_eq!(error.error_type, OpenAIErrorType::ServerError);
            assert_eq!(error.message, "agent failure");
        }
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

        assert_eq!(classified.error_type, OpenAIErrorType::ServerError);
        assert_eq!(classified.message, "internal failure");
    }

    #[test]
    fn classifies_oversized_prompt_as_invalid_request() {
        let error = OpenAIError::classify(&token_message(
            GeneratedTokenResult::PromptExceedsContextSize(OversizedPromptDetails {
                prompt_tokens: 9895,
                sequence_context_size: 8192,
            }),
        ))
        .unwrap();

        assert_eq!(error.error_type, OpenAIErrorType::InvalidRequestError);
        assert_eq!(
            error.message,
            "prompt has 9895 tokens but each agent sequence holds 8192 tokens; shorten the prompt or raise context_size"
        );
    }

    #[test]
    fn classifies_tool_call_validation_failure_as_server_error() {
        let classified = OpenAIError::classify(&token_message(
            GeneratedTokenResult::ToolCallValidationFailed(vec!["missing field x".to_owned()]),
        ))
        .unwrap();

        assert_eq!(classified.error_type, OpenAIErrorType::ServerError);
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
                GenerationSummary {
                    finish: GenerationFinish::EndOfGeneration,
                    usage: TokenUsage::new(),
                }
            )))
            .is_none()
        );
    }

    #[test]
    fn classifies_an_embedding_response_as_an_invalid_request() {
        let message = OutgoingMessage::Response(ResponseEnvelope {
            generated_by: None,
            request_id: "test-request".to_owned(),
            response: OutgoingResponse::Embedding(EmbeddingResult::Done),
        });

        let error = OpenAIError::classify(&message).unwrap();

        assert_eq!(error.error_type, OpenAIErrorType::InvalidRequestError);
        assert_eq!(
            error.message,
            "unexpected embedding response to a token generation request"
        );
    }

    #[test]
    fn does_not_classify_a_parsed_tool_call() {
        let parsed = vec![ParsedToolCall::new(
            "call_x".to_owned(),
            "get_weather".to_owned(),
            ToolCallArguments::ValidJson(json!({"location": "Paris"})),
        )];

        assert!(
            OpenAIError::classify(&token_message(GeneratedTokenResult::ToolCallParsed(parsed)))
                .is_none()
        );
    }
}
