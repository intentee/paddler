use std::sync::Arc;

use async_trait::async_trait;
use llama_cpp_bindings_types::ParsedToolCall;
use parking_lot::Mutex;
use serde_json::to_string;

use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::generation_summary::GenerationSummary;
use paddler_messaging::inference_client::message::Message as OutgoingMessage;
use paddler_messaging::inference_client::response::Response as OutgoingResponse;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;

use crate::agent_relay_error::AgentRelayError;
use crate::chunk_forwarding_session_controller::transform_result::TransformResult;
use crate::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;
use crate::compatibility::openai_service::chat_completion_chunk::ChatCompletionChunk;
use crate::compatibility::openai_service::chat_completion_chunk_choice::ChatCompletionChunkChoice;
use crate::compatibility::openai_service::chat_completion_chunk_payload::ChatCompletionChunkPayload;
use crate::compatibility::openai_service::chat_completion_finish_reason::chat_completion_finish_reason;
use crate::compatibility::openai_service::openai_streaming_state::OpenAIStreamingState;
use crate::compatibility::openai_service::try_universal_error_chunk::try_universal_error_chunk;

#[derive(Clone)]
pub struct OpenAIStreamingResponseTransformer {
    pub created: u64,
    pub include_usage: bool,
    pub model: String,
    pub state: Arc<Mutex<OpenAIStreamingState>>,
    pub system_fingerprint: String,
}

impl OpenAIStreamingResponseTransformer {
    fn chunk(
        &self,
        request_id: &str,
        payload: ChatCompletionChunkPayload<'_>,
    ) -> Result<String, AgentRelayError> {
        to_string(&ChatCompletionChunk {
            created: self.created,
            id: request_id,
            model: &self.model,
            payload,
            system_fingerprint: &self.system_fingerprint,
        })
        .map_err(AgentRelayError::MessageUnserializable)
    }

    fn handle_content(
        &self,
        request_id: &str,
        text: &str,
    ) -> Result<Vec<TransformResult>, AgentRelayError> {
        self.chunk(
            request_id,
            ChatCompletionChunkPayload::Choice(ChatCompletionChunkChoice::Content(text)),
        )
        .map(|chunk| vec![TransformResult::Chunk(chunk)])
    }

    fn handle_tool_call_parsed(
        &self,
        request_id: &str,
        parsed_calls: &[ParsedToolCall],
    ) -> Result<Vec<TransformResult>, AgentRelayError> {
        if parsed_calls.is_empty() {
            return Ok(vec![]);
        }

        self.state.lock().saw_tool_call = true;

        self.chunk(
            request_id,
            ChatCompletionChunkPayload::Choice(ChatCompletionChunkChoice::ToolCalls(parsed_calls)),
        )
        .map(|chunk| vec![TransformResult::Chunk(chunk)])
    }

    fn handle_done(
        &self,
        request_id: &str,
        summary: &GenerationSummary,
    ) -> Result<Vec<TransformResult>, AgentRelayError> {
        let saw_tool_call = self.state.lock().saw_tool_call;
        let finish_reason = chat_completion_finish_reason(summary.finish, saw_tool_call);

        self.chunk(
            request_id,
            ChatCompletionChunkPayload::Choice(ChatCompletionChunkChoice::Finish(finish_reason)),
        )
        .and_then(|finish_chunk| {
            let finish = TransformResult::Chunk(finish_chunk);

            if self.include_usage {
                self.chunk(
                    request_id,
                    ChatCompletionChunkPayload::Usage(&summary.usage),
                )
                .map(|usage_chunk| vec![finish, TransformResult::Chunk(usage_chunk)])
            } else {
                Ok(vec![finish])
            }
        })
    }
}

#[async_trait]
impl TransformsOutgoingMessage for OpenAIStreamingResponseTransformer {
    type Output = TransformResult;

    async fn transform(
        &self,
        message: OutgoingMessage,
    ) -> Result<Vec<TransformResult>, AgentRelayError> {
        match message {
            OutgoingMessage::Response(ResponseEnvelope {
                request_id,
                response:
                    OutgoingResponse::GeneratedToken(
                        GeneratedTokenResult::ContentToken(text)
                        | GeneratedTokenResult::UndeterminableToken(text),
                    ),
                ..
            }) => self.handle_content(&request_id, &text),
            OutgoingMessage::Response(ResponseEnvelope {
                response:
                    OutgoingResponse::GeneratedToken(
                        GeneratedTokenResult::ReasoningToken(_)
                        | GeneratedTokenResult::ToolCallToken(_),
                    ),
                ..
            }) => Ok(vec![]),
            OutgoingMessage::Response(ResponseEnvelope {
                request_id,
                response:
                    OutgoingResponse::GeneratedToken(GeneratedTokenResult::ToolCallParsed(parsed_calls)),
                ..
            }) => self.handle_tool_call_parsed(&request_id, &parsed_calls),
            OutgoingMessage::Response(ResponseEnvelope {
                request_id,
                response: OutgoingResponse::GeneratedToken(GeneratedTokenResult::Done(summary)),
                ..
            }) => self.handle_done(&request_id, &summary),
            other => try_universal_error_chunk(&other)
                .map(|error_chunk| vec![error_chunk])
                .ok_or_else(|| AgentRelayError::MessageNotRelayable {
                    message: Box::new(other),
                }),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use llama_cpp_bindings_types::ParsedToolCall;
    use llama_cpp_bindings_types::TokenUsage;
    use llama_cpp_bindings_types::ToolCallArguments;
    use parking_lot::Mutex;
    use serde_json::json;

    use paddler_messaging::embedding_result::EmbeddingResult;
    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use paddler_messaging::generation_finish::GenerationFinish;
    use paddler_messaging::generation_summary::GenerationSummary;
    use paddler_messaging::inference_client::message::Message as OutgoingMessage;
    use paddler_messaging::inference_client::notification::Notification;
    use paddler_messaging::inference_client::response::Response as OutgoingResponse;
    use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
    use paddler_messaging::jsonrpc::error_envelope::ErrorEnvelope;
    use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
    use paddler_messaging::oversized_media_details::OversizedMediaDetails;
    use paddler_messaging::raw_tool_call_tokens::RawToolCallTokens;

    use super::OpenAIStreamingResponseTransformer;
    use super::OpenAIStreamingState;
    use crate::agent_relay_error::AgentRelayError;
    use crate::chunk_forwarding_session_controller::transform_result::TransformResult;
    use crate::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;

    #[must_use]
    pub fn token_message(token_result: GeneratedTokenResult) -> OutgoingMessage {
        OutgoingMessage::Response(ResponseEnvelope {
            generated_by: None,
            request_id: "test-request".to_owned(),
            response: OutgoingResponse::GeneratedToken(token_result),
        })
    }

    #[must_use]
    pub fn error_message(code: i32, description: &str) -> OutgoingMessage {
        OutgoingMessage::Error(ErrorEnvelope {
            request_id: "test-request".to_owned(),
            error: JsonRpcError {
                code,
                description: description.to_owned(),
            },
        })
    }

    #[must_use]
    pub fn response_message(response: OutgoingResponse) -> OutgoingMessage {
        OutgoingMessage::Response(ResponseEnvelope {
            generated_by: None,
            request_id: "test-request".to_owned(),
            response,
        })
    }

    #[must_use]
    pub fn summary_with_counts(
        prompt_tokens: u64,
        content_tokens: u64,
        reasoning_tokens: u64,
    ) -> GenerationSummary {
        GenerationSummary {
            finish: GenerationFinish::EndOfGeneration,
            usage: TokenUsage {
                prompt_tokens,
                content_tokens,
                reasoning_tokens,
                ..TokenUsage::default()
            },
        }
    }

    #[must_use]
    pub fn weather_call() -> ParsedToolCall {
        ParsedToolCall::new(
            "call_x".to_owned(),
            "get_weather".to_owned(),
            ToolCallArguments::ValidJson(json!({ "location": "Paris" })),
        )
    }

    #[must_use]
    pub fn invalid_json_call() -> ParsedToolCall {
        ParsedToolCall::new(
            "call_invalid".to_owned(),
            "broken_tool".to_owned(),
            ToolCallArguments::InvalidJson("{not valid json".to_owned()),
        )
    }

    fn streaming_transformer(include_usage: bool) -> OpenAIStreamingResponseTransformer {
        OpenAIStreamingResponseTransformer {
            created: 0,
            include_usage,
            model: "test-model".to_owned(),
            state: Arc::new(Mutex::new(OpenAIStreamingState::default())),
            system_fingerprint: "test-fingerprint".to_owned(),
        }
    }

    #[tokio::test]
    async fn streaming_content_token_emits_content_delta() {
        let transformer = streaming_transformer(false);

        let message = token_message(GeneratedTokenResult::ContentToken("hello".to_owned()));
        let chunks = transformer
            .transform(message)
            .await
            .expect("the transformer must accept the message");

        assert_eq!(
            chunks,
            vec![
                TransformResult::Chunk(r#"{"id":"test-request","object":"chat.completion.chunk","created":0,"model":"test-model","system_fingerprint":"test-fingerprint","choices":[{"index":0,"delta":{"role":"assistant","content":"hello"},"logprobs":null,"finish_reason":null}]}"#.to_owned()),
            ]
        );
    }

    #[tokio::test]
    async fn streaming_reasoning_token_is_dropped() {
        let transformer = streaming_transformer(false);

        let message = token_message(GeneratedTokenResult::ReasoningToken("thought".to_owned()));
        let chunks = transformer
            .transform(message)
            .await
            .expect("the transformer must accept the message");

        assert_eq!(chunks.len(), 0);
    }

    #[tokio::test]
    async fn streaming_undeterminable_token_emits_content_delta() {
        let transformer = streaming_transformer(false);

        let message = token_message(GeneratedTokenResult::UndeterminableToken(
            "ambig".to_owned(),
        ));
        let chunks = transformer
            .transform(message)
            .await
            .expect("the transformer must accept the message");

        assert_eq!(
            chunks,
            vec![
                TransformResult::Chunk(r#"{"id":"test-request","object":"chat.completion.chunk","created":0,"model":"test-model","system_fingerprint":"test-fingerprint","choices":[{"index":0,"delta":{"role":"assistant","content":"ambig"},"logprobs":null,"finish_reason":null}]}"#.to_owned()),
            ]
        );
    }

    #[tokio::test]
    async fn streaming_tool_call_token_is_silently_dropped() {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::ToolCallToken(
                "{".to_owned(),
            )))
            .await
            .expect("the transformer must accept the message");

        assert_eq!(chunks.len(), 0);
    }

    #[tokio::test]
    async fn streaming_tool_call_parsed_emits_structured_tool_calls_chunk() {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::ToolCallParsed(vec![
                weather_call(),
            ])))
            .await
            .expect("the transformer must accept the message");

        assert_eq!(
            chunks,
            vec![
                TransformResult::Chunk(r#"{"id":"test-request","object":"chat.completion.chunk","created":0,"model":"test-model","system_fingerprint":"test-fingerprint","choices":[{"index":0,"delta":{"role":"assistant","tool_calls":[{"index":0,"id":"call_x","type":"function","function":{"name":"get_weather","arguments":"{\"location\":\"Paris\"}"}}]},"logprobs":null,"finish_reason":null}]}"#.to_owned()),
            ]
        );
    }

    #[tokio::test]
    async fn streaming_done_after_tool_call_uses_tool_calls_finish_reason() {
        let transformer = streaming_transformer(false);

        transformer
            .transform(token_message(GeneratedTokenResult::ToolCallParsed(vec![
                weather_call(),
            ])))
            .await
            .expect("the transformer must accept the message");

        let summary = summary_with_counts(2, 0, 0);
        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::Done(summary)))
            .await
            .expect("the transformer must accept the message");

        assert_eq!(
            chunks,
            vec![
                TransformResult::Chunk(r#"{"id":"test-request","object":"chat.completion.chunk","created":0,"model":"test-model","system_fingerprint":"test-fingerprint","choices":[{"index":0,"delta":{},"logprobs":null,"finish_reason":"tool_calls"}]}"#.to_owned()),
            ]
        );
    }

    #[tokio::test]
    async fn streaming_done_without_tool_call_uses_stop_finish_reason() {
        let transformer = streaming_transformer(false);

        transformer
            .transform(token_message(GeneratedTokenResult::ContentToken(
                "hi".to_owned(),
            )))
            .await
            .expect("the transformer must accept the message");

        let summary = summary_with_counts(2, 1, 0);
        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::Done(summary)))
            .await
            .expect("the transformer must accept the message");

        assert_eq!(
            chunks,
            vec![
                TransformResult::Chunk(r#"{"id":"test-request","object":"chat.completion.chunk","created":0,"model":"test-model","system_fingerprint":"test-fingerprint","choices":[{"index":0,"delta":{},"logprobs":null,"finish_reason":"stop"}]}"#.to_owned()),
            ]
        );
    }

    #[tokio::test]
    async fn streaming_done_with_include_usage_emits_finish_then_usage_chunk() {
        let transformer = streaming_transformer(true);
        let summary = summary_with_counts(7, 4, 1);

        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::Done(summary)))
            .await
            .expect("the transformer must accept the message");

        assert_eq!(
            chunks,
            vec![
                TransformResult::Chunk(r#"{"id":"test-request","object":"chat.completion.chunk","created":0,"model":"test-model","system_fingerprint":"test-fingerprint","choices":[{"index":0,"delta":{},"logprobs":null,"finish_reason":"stop"}]}"#.to_owned()),
                TransformResult::Chunk(r#"{"id":"test-request","object":"chat.completion.chunk","created":0,"model":"test-model","system_fingerprint":"test-fingerprint","choices":[],"usage":{"prompt_tokens":7,"completion_tokens":5,"total_tokens":12,"prompt_tokens_details":{"cached_tokens":0,"audio_tokens":0},"completion_tokens_details":{"reasoning_tokens":1}}}"#.to_owned()),
            ]
        );
    }

    #[tokio::test]
    async fn streaming_done_without_include_usage_emits_only_finish_chunk() {
        let transformer = streaming_transformer(false);
        let summary = summary_with_counts(5, 3, 2);

        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::Done(summary)))
            .await
            .expect("the transformer must accept the message");

        assert_eq!(
            chunks,
            vec![
                TransformResult::Chunk(r#"{"id":"test-request","object":"chat.completion.chunk","created":0,"model":"test-model","system_fingerprint":"test-fingerprint","choices":[{"index":0,"delta":{},"logprobs":null,"finish_reason":"stop"}]}"#.to_owned()),
            ]
        );
    }

    #[tokio::test]
    async fn streaming_tool_call_parse_failed_emits_server_error() {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::ToolCallParseFailed(
                "bad payload".to_owned(),
            )))
            .await
            .expect("the transformer must accept the message");

        assert_eq!(
            chunks,
            vec![
                TransformResult::Error(r#"{"error":{"message":"bad payload","type":"server_error","param":null,"code":null}}"#.to_owned()),
            ]
        );
    }

    #[tokio::test]
    async fn streaming_tool_call_validation_failed_emits_server_error() {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(token_message(
                GeneratedTokenResult::ToolCallValidationFailed(vec!["missing field x".to_owned()]),
            ))
            .await
            .expect("the transformer must accept the message");

        assert_eq!(
            chunks,
            vec![
                TransformResult::Error(r#"{"error":{"message":"missing field x","type":"server_error","param":null,"code":null}}"#.to_owned()),
            ]
        );
    }

    #[tokio::test]
    async fn streaming_unrecognized_tool_call_format_emits_server_error() {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(token_message(
                GeneratedTokenResult::UnrecognizedToolCallFormat(RawToolCallTokens {
                    text: "<unknown_marker>blah</unknown_marker>".to_owned(),
                    ffi_error_message: "common_chat_parse failed: no parser".to_owned(),
                }),
            ))
            .await
            .expect("the transformer must accept the message");

        assert_eq!(
            chunks,
            vec![
                TransformResult::Error(r#"{"error":{"message":"model produced output the parser did not recognise as any registered tool-call format; FFI error: common_chat_parse failed: no parser; raw text: <unknown_marker>blah</unknown_marker>","type":"server_error","param":null,"code":null}}"#.to_owned()),
            ]
        );
    }

    #[tokio::test]
    async fn streaming_error_message_returns_server_error() {
        let transformer = streaming_transformer(false);

        let message = error_message(500, "internal server error");
        let chunks = transformer
            .transform(message)
            .await
            .expect("the transformer must accept the message");

        assert_eq!(
            chunks,
            vec![
                TransformResult::Error(r#"{"error":{"message":"internal server error","type":"server_error","param":null,"code":null}}"#.to_owned()),
            ]
        );
    }

    #[tokio::test]
    async fn streaming_chat_template_error_returns_server_error() {
        let transformer = streaming_transformer(false);

        let message = token_message(GeneratedTokenResult::ChatTemplateError(
            "bad template".to_owned(),
        ));
        let chunks = transformer
            .transform(message)
            .await
            .expect("the transformer must accept the message");

        assert_eq!(
            chunks,
            vec![
                TransformResult::Error(r#"{"error":{"message":"bad template","type":"server_error","param":null,"code":null}}"#.to_owned()),
            ]
        );
    }

    #[tokio::test]
    async fn streaming_image_decoding_failed_returns_invalid_request_error() {
        let transformer = streaming_transformer(false);

        let message = token_message(GeneratedTokenResult::ImageDecodingFailed(
            "unsupported format".to_owned(),
        ));
        let chunks = transformer
            .transform(message)
            .await
            .expect("the transformer must accept the message");

        assert_eq!(
            chunks,
            vec![
                TransformResult::Error(r#"{"error":{"message":"unsupported format","type":"invalid_request_error","param":null,"code":null}}"#.to_owned()),
            ]
        );
    }

    #[tokio::test]
    async fn streaming_multimodal_not_supported_returns_invalid_request_error() {
        let transformer = streaming_transformer(false);

        let message = token_message(GeneratedTokenResult::MultimodalNotSupported(
            "model does not support images".to_owned(),
        ));
        let chunks = transformer
            .transform(message)
            .await
            .expect("the transformer must accept the message");

        assert_eq!(
            chunks,
            vec![
                TransformResult::Error(r#"{"error":{"message":"model does not support images","type":"invalid_request_error","param":null,"code":null}}"#.to_owned()),
            ]
        );
    }

    #[tokio::test]
    async fn streaming_media_exceeding_the_micro_batch_returns_invalid_request_error() {
        let transformer = streaming_transformer(false);

        let message = token_message(GeneratedTokenResult::MediaExceedsMicroBatch(
            OversizedMediaDetails {
                media_tokens: 256,
                micro_batch_tokens: 128,
            },
        ));
        let chunks = transformer
            .transform(message)
            .await
            .expect("the transformer must accept the message");

        assert_eq!(
            chunks,
            vec![
                TransformResult::Error(r#"{"error":{"message":"media required 256 tokens but one agent micro batch holds 128 tokens","type":"invalid_request_error","param":null,"code":null}}"#.to_owned()),
            ]
        );
    }

    #[tokio::test]
    async fn streaming_tool_call_with_invalid_json_arguments_passes_raw_string_through() {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::ToolCallParsed(vec![
                invalid_json_call(),
            ])))
            .await
            .unwrap();

        assert_eq!(
            chunks,
            vec![
                TransformResult::Chunk(r#"{"id":"test-request","object":"chat.completion.chunk","created":0,"model":"test-model","system_fingerprint":"test-fingerprint","choices":[{"index":0,"delta":{"role":"assistant","tool_calls":[{"index":0,"id":"call_invalid","type":"function","function":{"name":"broken_tool","arguments":"{not valid json"}}]},"logprobs":null,"finish_reason":null}]}"#.to_owned()),
            ]
        );
    }

    #[tokio::test]
    async fn streaming_empty_parsed_tool_calls_emit_no_chunks() {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::ToolCallParsed(
                Vec::new(),
            )))
            .await
            .unwrap();

        assert_eq!(chunks.len(), 0);
    }

    #[tokio::test]
    async fn streaming_embedding_response_returns_invalid_request_error() {
        let transformer = streaming_transformer(false);

        let message = response_message(OutgoingResponse::Embedding(EmbeddingResult::Done));
        let chunks = transformer.transform(message).await.unwrap();

        assert_eq!(
            chunks,
            vec![
                TransformResult::Error(r#"{"error":{"message":"unexpected embedding response to a token generation request","type":"invalid_request_error","param":null,"code":null}}"#.to_owned()),
            ]
        );
    }

    #[tokio::test]
    async fn streaming_grammar_incompatible_with_thinking_returns_invalid_request_error() {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(token_message(
                GeneratedTokenResult::GrammarIncompatibleWithThinking(
                    "grammar conflicts with thinking".to_owned(),
                ),
            ))
            .await
            .unwrap();

        assert_eq!(
            chunks,
            vec![
                TransformResult::Error(r#"{"error":{"message":"grammar conflicts with thinking","type":"invalid_request_error","param":null,"code":null}}"#.to_owned()),
            ]
        );
    }

    #[tokio::test]
    async fn streaming_grammar_rejected_model_output_returns_server_error() {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(token_message(
                GeneratedTokenResult::GrammarRejectedModelOutput(
                    "output rejected by grammar".to_owned(),
                ),
            ))
            .await
            .unwrap();

        assert_eq!(
            chunks,
            vec![
                TransformResult::Error(r#"{"error":{"message":"output rejected by grammar","type":"server_error","param":null,"code":null}}"#.to_owned()),
            ]
        );
    }

    #[tokio::test]
    async fn streaming_grammar_initialization_failed_returns_server_error() {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(token_message(
                GeneratedTokenResult::GrammarInitializationFailed(
                    "could not build grammar".to_owned(),
                ),
            ))
            .await
            .unwrap();

        assert_eq!(
            chunks,
            vec![
                TransformResult::Error(r#"{"error":{"message":"could not build grammar","type":"server_error","param":null,"code":null}}"#.to_owned()),
            ]
        );
    }

    #[tokio::test]
    async fn streaming_grammar_syntax_error_returns_invalid_request_error() {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::GrammarSyntaxError(
                "bad grammar syntax".to_owned(),
            )))
            .await
            .unwrap();

        assert_eq!(
            chunks,
            vec![
                TransformResult::Error(r#"{"error":{"message":"bad grammar syntax","type":"invalid_request_error","param":null,"code":null}}"#.to_owned()),
            ]
        );
    }

    #[tokio::test]
    async fn streaming_sampler_error_returns_server_error() {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::SamplerError(
                "sampler blew up".to_owned(),
            )))
            .await
            .unwrap();

        assert_eq!(
            chunks,
            vec![
                TransformResult::Error(r#"{"error":{"message":"sampler blew up","type":"server_error","param":null,"code":null}}"#.to_owned()),
            ]
        );
    }

    #[tokio::test]
    async fn streaming_tool_schema_invalid_returns_invalid_request_error() {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::ToolSchemaInvalid(
                "schema is not valid".to_owned(),
            )))
            .await
            .unwrap();

        assert_eq!(
            chunks,
            vec![
                TransformResult::Error(r#"{"error":{"message":"schema is not valid","type":"invalid_request_error","param":null,"code":null}}"#.to_owned()),
            ]
        );
    }

    #[tokio::test]
    async fn rejects_inference_socket_notifications() {
        let transform_result = streaming_transformer(false)
            .transform(OutgoingMessage::Notification(
                Notification::TokenGenerationEnabled,
            ))
            .await;

        assert!(matches!(
            transform_result,
            Err(AgentRelayError::MessageNotRelayable { message })
                if matches!(*message, OutgoingMessage::Notification(Notification::TokenGenerationEnabled))
        ));
    }
}
