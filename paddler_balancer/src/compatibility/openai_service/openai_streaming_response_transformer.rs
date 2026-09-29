use std::sync::Arc;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use async_trait::async_trait;
use llama_cpp_bindings_types::ParsedToolCall;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::generation_summary::GenerationSummary;
use paddler_messaging::inference_client::message::Message as OutgoingMessage;
use paddler_messaging::inference_client::response::Response as OutgoingResponse;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use parking_lot::Mutex;

use crate::chunk_forwarding_session_controller::transform_result::TransformResult;
use crate::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;
use crate::compatibility::openai_service::chat_completion_chunk::ChatCompletionChunk;
use crate::compatibility::openai_service::chat_completion_chunk_choice::ChatCompletionChunkChoice;
use crate::compatibility::openai_service::chat_completion_chunk_payload::ChatCompletionChunkPayload;
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
    fn chunk(&self, request_id: &str, payload: ChatCompletionChunkPayload<'_>) -> Result<String> {
        serde_json::to_string(&ChatCompletionChunk {
            created: self.created,
            id: request_id,
            model: &self.model,
            payload,
            system_fingerprint: &self.system_fingerprint,
        })
        .context("serializing chat completion chunk")
    }

    fn handle_content(&self, request_id: &str, text: &str) -> Result<Vec<TransformResult>> {
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
    ) -> Result<Vec<TransformResult>> {
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
    ) -> Result<Vec<TransformResult>> {
        let saw_tool_call = self.state.lock().saw_tool_call;
        let finish_reason = if saw_tool_call { "tool_calls" } else { "stop" };

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

    async fn transform(&self, message: OutgoingMessage) -> Result<Vec<TransformResult>> {
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
                .ok_or_else(|| {
                    anyhow!(
                        "OpenAIStreamingResponseTransformer received an outgoing message it does not know how to handle: {other:?}"
                    )
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
    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use paddler_messaging::generation_summary::GenerationSummary;
    use paddler_messaging::inference_client::message::Message as OutgoingMessage;
    use paddler_messaging::inference_client::notification::Notification;
    use paddler_messaging::inference_client::response::Response as OutgoingResponse;
    use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
    use paddler_messaging::jsonrpc::error_envelope::ErrorEnvelope;
    use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
    use parking_lot::Mutex;
    use serde_json::json;

    use crate::chunk_forwarding_session_controller::transform_result::TransformResult;
    use crate::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;

    use super::OpenAIStreamingResponseTransformer;
    use super::OpenAIStreamingState;

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

    pub fn assert_chunk_body_does_not_contain(result: &TransformResult, unexpected: &str) {
        assert!(
            matches!(result, TransformResult::Chunk(content) if !content.contains(unexpected)),
            "expected a chunk without '{unexpected}': {result:?}"
        );
    }

    pub fn assert_chunk_body_contains(result: &TransformResult, expected: &str) {
        assert!(
            matches!(result, TransformResult::Chunk(content) if content.contains(expected)),
            "expected a chunk containing '{expected}': {result:?}"
        );
    }

    pub fn assert_chunk_body_equals(result: &TransformResult, expected: &str) {
        assert!(
            matches!(result, TransformResult::Chunk(content) if content == expected),
            "expected the chunk {expected}: {result:?}"
        );
    }

    pub fn assert_error_body_contains(result: &TransformResult, expected: &str) {
        assert!(
            matches!(result, TransformResult::Error(content) if content.contains(expected)),
            "expected an error containing '{expected}': {result:?}"
        );
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

        assert_eq!(chunks.len(), 1);
        assert_chunk_body_equals(
            &chunks[0],
            r#"{"id":"test-request","object":"chat.completion.chunk","created":0,"model":"test-model","system_fingerprint":"test-fingerprint","choices":[{"index":0,"delta":{"role":"assistant","content":"hello"},"logprobs":null,"finish_reason":null}]}"#,
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

        assert_eq!(chunks.len(), 1);
        assert_chunk_body_contains(&chunks[0], "\"content\":\"ambig\"");
        assert_chunk_body_does_not_contain(&chunks[0], "reasoning_content");
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

        assert_eq!(chunks.len(), 1);
        assert_chunk_body_equals(
            &chunks[0],
            r#"{"id":"test-request","object":"chat.completion.chunk","created":0,"model":"test-model","system_fingerprint":"test-fingerprint","choices":[{"index":0,"delta":{"role":"assistant","tool_calls":[{"index":0,"id":"call_x","type":"function","function":{"name":"get_weather","arguments":"{\"location\":\"Paris\"}"}}]},"logprobs":null,"finish_reason":null}]}"#,
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

        assert_eq!(chunks.len(), 1);
        assert_chunk_body_contains(&chunks[0], "\"finish_reason\":\"tool_calls\"");
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

        assert_eq!(chunks.len(), 1);
        assert_chunk_body_contains(&chunks[0], "\"finish_reason\":\"stop\"");
    }

    #[tokio::test]
    async fn streaming_done_with_include_usage_emits_finish_then_usage_chunk() {
        let transformer = streaming_transformer(true);
        let summary = summary_with_counts(7, 4, 1);

        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::Done(summary)))
            .await
            .expect("the transformer must accept the message");

        assert_eq!(chunks.len(), 2);
        assert_chunk_body_contains(&chunks[0], "\"finish_reason\":\"stop\"");
        assert_chunk_body_does_not_contain(&chunks[0], "usage");
        assert_chunk_body_equals(
            &chunks[1],
            r#"{"id":"test-request","object":"chat.completion.chunk","created":0,"model":"test-model","system_fingerprint":"test-fingerprint","choices":[],"usage":{"prompt_tokens":7,"completion_tokens":5,"total_tokens":12,"prompt_tokens_details":{"cached_tokens":0,"audio_tokens":0},"completion_tokens_details":{"reasoning_tokens":1}}}"#,
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

        assert_eq!(chunks.len(), 1);
        assert_chunk_body_equals(
            &chunks[0],
            r#"{"id":"test-request","object":"chat.completion.chunk","created":0,"model":"test-model","system_fingerprint":"test-fingerprint","choices":[{"index":0,"delta":{},"logprobs":null,"finish_reason":"stop"}]}"#,
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

        assert_eq!(chunks.len(), 1);
        assert_error_body_contains(&chunks[0], "bad payload");
        assert_error_body_contains(&chunks[0], "server_error");
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

        assert_eq!(chunks.len(), 1);
        assert_error_body_contains(&chunks[0], "missing field x");
    }

    #[tokio::test]
    async fn streaming_unrecognized_tool_call_format_emits_server_error() {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(token_message(
                GeneratedTokenResult::UnrecognizedToolCallFormat(
                    paddler_messaging::raw_tool_call_tokens::RawToolCallTokens {
                        text: "<unknown_marker>blah</unknown_marker>".to_owned(),
                        ffi_error_message: "common_chat_parse failed: no parser".to_owned(),
                    },
                ),
            ))
            .await
            .expect("the transformer must accept the message");

        assert_eq!(chunks.len(), 1);
        assert_error_body_contains(&chunks[0], "common_chat_parse failed: no parser");
        assert_error_body_contains(&chunks[0], "<unknown_marker>blah</unknown_marker>");
        assert_error_body_contains(&chunks[0], "server_error");
    }

    #[tokio::test]
    async fn streaming_error_message_returns_error_variant() {
        let transformer = streaming_transformer(false);

        let message = error_message(500, "internal server error");
        let chunks = transformer
            .transform(message)
            .await
            .expect("the transformer must accept the message");

        assert_eq!(chunks.len(), 1);
        assert_error_body_contains(&chunks[0], "internal server error");
        assert_error_body_contains(&chunks[0], "server_error");
    }

    #[tokio::test]
    async fn streaming_chat_template_error_returns_error_variant() {
        let transformer = streaming_transformer(false);

        let message = token_message(GeneratedTokenResult::ChatTemplateError(
            "bad template".to_owned(),
        ));
        let chunks = transformer
            .transform(message)
            .await
            .expect("the transformer must accept the message");

        assert_eq!(chunks.len(), 1);
        assert_error_body_contains(&chunks[0], "bad template");
        assert_error_body_contains(&chunks[0], "server_error");
    }

    #[tokio::test]
    async fn streaming_image_decoding_failed_returns_error_variant() {
        let transformer = streaming_transformer(false);

        let message = token_message(GeneratedTokenResult::ImageDecodingFailed(
            "unsupported format".to_owned(),
        ));
        let chunks = transformer
            .transform(message)
            .await
            .expect("the transformer must accept the message");

        assert_eq!(chunks.len(), 1);
        assert_error_body_contains(&chunks[0], "unsupported format");
        assert_error_body_contains(&chunks[0], "server_error");
    }

    #[tokio::test]
    async fn streaming_multimodal_not_supported_returns_error_variant() {
        let transformer = streaming_transformer(false);

        let message = token_message(GeneratedTokenResult::MultimodalNotSupported(
            "model does not support images".to_owned(),
        ));
        let chunks = transformer
            .transform(message)
            .await
            .expect("the transformer must accept the message");

        assert_eq!(chunks.len(), 1);
        assert_error_body_contains(&chunks[0], "model does not support images");
        assert_error_body_contains(&chunks[0], "server_error");
    }

    #[tokio::test]
    async fn streaming_image_exceeds_batch_size_returns_error_variant() {
        let transformer = streaming_transformer(false);

        let message = token_message(GeneratedTokenResult::ImageExceedsBatchSize(
            paddler_messaging::oversized_image_details::OversizedImageDetails {
                image_tokens: 368,
                n_batch: 100,
            },
        ));
        let chunks = transformer
            .transform(message)
            .await
            .expect("the transformer must accept the message");

        assert_eq!(chunks.len(), 1);
        assert_error_body_contains(&chunks[0], "368");
        assert_error_body_contains(&chunks[0], "100");
        assert_error_body_contains(&chunks[0], "server_error");
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

        assert_eq!(chunks.len(), 1);
        assert_chunk_body_contains(&chunks[0], "{not valid json");
        assert_chunk_body_contains(&chunks[0], "\"name\":\"broken_tool\"");
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

        let message = response_message(OutgoingResponse::Embedding(
            paddler_messaging::embedding_result::EmbeddingResult::Done,
        ));
        let chunks = transformer.transform(message).await.unwrap();

        assert_eq!(chunks.len(), 1);
        assert_error_body_contains(&chunks[0], "invalid_request_error");
        assert_error_body_contains(
            &chunks[0],
            "unexpected embedding response in chat completions",
        );
    }

    #[tokio::test]
    async fn streaming_grammar_incompatible_with_thinking_returns_server_error() {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(token_message(
                GeneratedTokenResult::GrammarIncompatibleWithThinking(
                    "grammar conflicts with thinking".to_owned(),
                ),
            ))
            .await
            .unwrap();

        assert_eq!(chunks.len(), 1);
        assert_error_body_contains(&chunks[0], "grammar conflicts with thinking");
        assert_error_body_contains(&chunks[0], "server_error");
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

        assert_eq!(chunks.len(), 1);
        assert_error_body_contains(&chunks[0], "output rejected by grammar");
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

        assert_eq!(chunks.len(), 1);
        assert_error_body_contains(&chunks[0], "could not build grammar");
    }

    #[tokio::test]
    async fn streaming_grammar_syntax_error_returns_server_error() {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::GrammarSyntaxError(
                "bad grammar syntax".to_owned(),
            )))
            .await
            .unwrap();

        assert_eq!(chunks.len(), 1);
        assert_error_body_contains(&chunks[0], "bad grammar syntax");
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

        assert_eq!(chunks.len(), 1);
        assert_error_body_contains(&chunks[0], "sampler blew up");
    }

    #[tokio::test]
    async fn streaming_tool_schema_invalid_returns_server_error() {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::ToolSchemaInvalid(
                "schema is not valid".to_owned(),
            )))
            .await
            .unwrap();

        assert_eq!(chunks.len(), 1);
        assert_error_body_contains(&chunks[0], "schema is not valid");
    }

    #[tokio::test]
    async fn rejects_inference_socket_notifications() {
        assert!(
            streaming_transformer(false)
                .transform(OutgoingMessage::Notification(
                    Notification::TokenGenerationEnabled
                ))
                .await
                .is_err()
        );
    }
}
