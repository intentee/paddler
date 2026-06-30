use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use llama_cpp_bindings_types::ParsedToolCall;
use llama_cpp_bindings_types::TokenUsage;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::generation_summary::GenerationSummary;
use paddler_messaging::inference_client::message::Message as OutgoingMessage;
use paddler_messaging::inference_client::response::Response as OutgoingResponse;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use parking_lot::Mutex;
use serde_json::json;

use crate::chunk_forwarding_session_controller::transform_result::TransformResult;
use crate::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;
use crate::compatibility::openai_service::arguments_to_tool_call_string::arguments_to_tool_call_string;
use crate::compatibility::openai_service::openai_streaming_state::OpenAIStreamingState;
use crate::compatibility::openai_service::openai_usage_json::openai_usage_json;
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
    fn content_chunk(&self, request_id: &str, text: &str) -> String {
        json!({
            "id": request_id,
            "object": "chat.completion.chunk",
            "created": self.created,
            "model": self.model,
            "system_fingerprint": self.system_fingerprint,
            "choices": [
                {
                    "index": 0,
                    "delta": {
                        "role": "assistant",
                        "content": text,
                    },
                    "logprobs": null,
                    "finish_reason": null
                }
            ]
        })
        .to_string()
    }

    fn tool_calls_chunk(&self, request_id: &str, parsed_calls: &[ParsedToolCall]) -> String {
        let tool_calls = parsed_calls
            .iter()
            .enumerate()
            .map(|(index, call)| {
                json!({
                    "index": index,
                    "id": call.id,
                    "type": "function",
                    "function": {
                        "name": call.name,
                        "arguments": arguments_to_tool_call_string(&call.arguments),
                    }
                })
            })
            .collect::<Vec<_>>();

        json!({
            "id": request_id,
            "object": "chat.completion.chunk",
            "created": self.created,
            "model": self.model,
            "system_fingerprint": self.system_fingerprint,
            "choices": [
                {
                    "index": 0,
                    "delta": {
                        "role": "assistant",
                        "tool_calls": tool_calls,
                    },
                    "logprobs": null,
                    "finish_reason": null
                }
            ]
        })
        .to_string()
    }

    fn finish_chunk(&self, request_id: &str, finish_reason: &str) -> String {
        json!({
            "id": request_id,
            "object": "chat.completion.chunk",
            "created": self.created,
            "model": self.model,
            "system_fingerprint": self.system_fingerprint,
            "choices": [
                {
                    "index": 0,
                    "delta": {},
                    "logprobs": null,
                    "finish_reason": finish_reason
                }
            ]
        })
        .to_string()
    }

    fn usage_chunk(&self, request_id: &str, usage: &TokenUsage) -> String {
        json!({
            "id": request_id,
            "object": "chat.completion.chunk",
            "created": self.created,
            "model": self.model,
            "system_fingerprint": self.system_fingerprint,
            "choices": [],
            "usage": openai_usage_json(usage),
        })
        .to_string()
    }

    fn handle_content(&self, request_id: &str, text: &str) -> Vec<TransformResult> {
        vec![TransformResult::Chunk(self.content_chunk(request_id, text))]
    }

    fn handle_tool_call_parsed(
        &self,
        request_id: &str,
        parsed_calls: &[ParsedToolCall],
    ) -> Vec<TransformResult> {
        if parsed_calls.is_empty() {
            return vec![];
        }

        self.state.lock().saw_tool_call = true;

        vec![TransformResult::Chunk(
            self.tool_calls_chunk(request_id, parsed_calls),
        )]
    }

    fn handle_done(&self, request_id: &str, summary: &GenerationSummary) -> Vec<TransformResult> {
        let saw_tool_call = self.state.lock().saw_tool_call;
        let finish_reason = if saw_tool_call { "tool_calls" } else { "stop" };

        let finish = TransformResult::Chunk(self.finish_chunk(request_id, finish_reason));

        if self.include_usage {
            vec![
                finish,
                TransformResult::Chunk(self.usage_chunk(request_id, &summary.usage)),
            ]
        } else {
            vec![finish]
        }
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
            }) => Ok(self.handle_content(&request_id, &text)),
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
            }) => Ok(self.handle_tool_call_parsed(&request_id, &parsed_calls)),
            OutgoingMessage::Response(ResponseEnvelope {
                request_id,
                response: OutgoingResponse::GeneratedToken(GeneratedTokenResult::Done(summary)),
                ..
            }) => Ok(self.handle_done(&request_id, &summary)),
            other => Ok(try_universal_error_chunk(&other).into_iter().collect()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use llama_cpp_bindings_types::ParsedToolCall;
    use llama_cpp_bindings_types::TokenUsage;
    use llama_cpp_bindings_types::ToolCallArguments;
    use paddler_messaging::embedding_result::EmbeddingResult;
    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use paddler_messaging::generation_summary::GenerationSummary;
    use paddler_messaging::inference_client::message::Message as OutgoingMessage;
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

    pub fn assert_chunk_contains(result: &TransformResult, expected: &str) {
        let content = result.chunk_body().expect("expected a chunk");

        assert!(
            content.contains(expected),
            "chunk does not contain '{expected}': {content}"
        );
    }

    pub fn assert_chunk_does_not_contain(result: &TransformResult, expected: &str) {
        let content = result.chunk_body().expect("expected a chunk");

        assert!(
            !content.contains(expected),
            "chunk unexpectedly contains '{expected}': {content}"
        );
    }

    pub fn assert_openai_error(
        result: &TransformResult,
        expected_type: &str,
        expected_message: &str,
    ) {
        let body = result.error_body().expect("expected an error");
        let envelope: serde_json::Value =
            serde_json::from_str(body).expect("error body must be valid JSON");

        assert_eq!(envelope["error"]["type"], expected_type);
        assert_eq!(envelope["error"]["message"], expected_message);
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
    async fn streaming_embedding_response_becomes_an_error_chunk() {
        let message = OutgoingMessage::Response(ResponseEnvelope {
            generated_by: None,
            request_id: "test-request".to_owned(),
            response: OutgoingResponse::Embedding(EmbeddingResult::Done),
        });

        let results = streaming_transformer(false)
            .transform(message)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_openai_error(
            &results[0],
            "invalid_request_error",
            "unexpected embedding response in chat completions",
        );
    }

    #[tokio::test]
    async fn streaming_content_token_emits_content_delta() {
        let transformer = streaming_transformer(false);

        let message = token_message(GeneratedTokenResult::ContentToken("hello".to_owned()));
        let chunks = transformer.transform(message).await.unwrap();

        assert_eq!(chunks.len(), 1);
        assert_chunk_contains(&chunks[0], "\"content\":\"hello\"");
        assert_chunk_contains(&chunks[0], "\"role\":\"assistant\"");
        assert_chunk_does_not_contain(&chunks[0], "reasoning_content");
    }

    #[tokio::test]
    async fn streaming_reasoning_token_is_dropped() {
        let transformer = streaming_transformer(false);

        let message = token_message(GeneratedTokenResult::ReasoningToken("thought".to_owned()));
        let chunks = transformer.transform(message).await.unwrap();

        assert_eq!(chunks.len(), 0);
    }

    #[tokio::test]
    async fn streaming_undeterminable_token_emits_content_delta() {
        let transformer = streaming_transformer(false);

        let message = token_message(GeneratedTokenResult::UndeterminableToken(
            "ambig".to_owned(),
        ));
        let chunks = transformer.transform(message).await.unwrap();

        assert_eq!(chunks.len(), 1);
        assert_chunk_contains(&chunks[0], "\"content\":\"ambig\"");
        assert_chunk_does_not_contain(&chunks[0], "reasoning_content");
    }

    #[tokio::test]
    async fn streaming_tool_call_token_is_silently_dropped() {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::ToolCallToken(
                "{".to_owned(),
            )))
            .await
            .unwrap();

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
            .unwrap();

        assert_eq!(chunks.len(), 1);
        assert_chunk_contains(&chunks[0], "\"tool_calls\"");
        assert_chunk_contains(&chunks[0], "\"id\":\"call_x\"");
        assert_chunk_contains(&chunks[0], "\"name\":\"get_weather\"");
        assert_chunk_contains(
            &chunks[0],
            "\"arguments\":\"{\\\"location\\\":\\\"Paris\\\"}\"",
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
            .unwrap();

        let summary = summary_with_counts(2, 0, 0);
        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::Done(summary)))
            .await
            .unwrap();

        assert_eq!(chunks.len(), 1);
        assert_chunk_contains(&chunks[0], "\"finish_reason\":\"tool_calls\"");
    }

    #[tokio::test]
    async fn streaming_done_without_tool_call_uses_stop_finish_reason() {
        let transformer = streaming_transformer(false);

        transformer
            .transform(token_message(GeneratedTokenResult::ContentToken(
                "hi".to_owned(),
            )))
            .await
            .unwrap();

        let summary = summary_with_counts(2, 1, 0);
        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::Done(summary)))
            .await
            .unwrap();

        assert_eq!(chunks.len(), 1);
        assert_chunk_contains(&chunks[0], "\"finish_reason\":\"stop\"");
    }

    #[tokio::test]
    async fn streaming_done_with_include_usage_emits_finish_then_usage_chunk() {
        let transformer = streaming_transformer(true);
        let summary = summary_with_counts(7, 4, 1);

        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::Done(summary)))
            .await
            .unwrap();

        assert_eq!(chunks.len(), 2);
        assert_chunk_contains(&chunks[0], "\"finish_reason\":\"stop\"");
        assert_chunk_does_not_contain(&chunks[0], "usage");
        assert_chunk_contains(&chunks[1], "\"prompt_tokens\":7");
        assert_chunk_contains(&chunks[1], "\"completion_tokens\":5");
        assert_chunk_contains(&chunks[1], "\"total_tokens\":12");
        assert_chunk_contains(&chunks[1], "\"choices\":[]");
    }

    #[tokio::test]
    async fn streaming_done_without_include_usage_emits_only_finish_chunk() {
        let transformer = streaming_transformer(false);
        let summary = summary_with_counts(5, 3, 2);

        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::Done(summary)))
            .await
            .unwrap();

        assert_eq!(chunks.len(), 1);
        assert_chunk_contains(&chunks[0], "\"finish_reason\":\"stop\"");
        assert_chunk_does_not_contain(&chunks[0], "usage");
    }

    #[tokio::test]
    async fn streaming_tool_call_parse_failed_emits_server_error() {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::ToolCallParseFailed(
                "bad payload".to_owned(),
            )))
            .await
            .unwrap();

        assert_eq!(chunks.len(), 1);
        assert_openai_error(&chunks[0], "server_error", "bad payload");
    }

    #[tokio::test]
    async fn streaming_tool_call_validation_failed_emits_server_error() {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(token_message(
                GeneratedTokenResult::ToolCallValidationFailed(vec!["missing field x".to_owned()]),
            ))
            .await
            .unwrap();

        assert_eq!(chunks.len(), 1);
        assert_openai_error(&chunks[0], "server_error", "missing field x");
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
            .unwrap();

        assert_eq!(chunks.len(), 1);
        assert_openai_error(
            &chunks[0],
            "server_error",
            "model produced output the parser did not recognise as any registered tool-call \
             format; FFI error: common_chat_parse failed: no parser; raw text: \
             <unknown_marker>blah</unknown_marker>",
        );
    }

    #[tokio::test]
    async fn streaming_error_message_returns_error_variant() {
        let transformer = streaming_transformer(false);

        let message = error_message(500, "internal server error");
        let chunks = transformer.transform(message).await.unwrap();

        assert_eq!(chunks.len(), 1);
        assert_openai_error(&chunks[0], "server_error", "internal server error");
    }

    #[tokio::test]
    async fn streaming_chat_template_error_returns_error_variant() {
        let transformer = streaming_transformer(false);

        let message = token_message(GeneratedTokenResult::ChatTemplateError(
            "bad template".to_owned(),
        ));
        let chunks = transformer.transform(message).await.unwrap();

        assert_eq!(chunks.len(), 1);
        assert_openai_error(&chunks[0], "server_error", "bad template");
    }

    #[tokio::test]
    async fn streaming_timeout_returns_error_variant() {
        let transformer = streaming_transformer(false);

        let message = response_message(OutgoingResponse::Timeout);
        let chunks = transformer.transform(message).await.unwrap();

        assert_eq!(chunks.len(), 1);
        assert_openai_error(&chunks[0], "timeout", "request timed out");
    }

    #[tokio::test]
    async fn streaming_too_many_buffered_requests_returns_error_variant() {
        let transformer = streaming_transformer(false);

        let message = response_message(OutgoingResponse::TooManyBufferedRequests);
        let chunks = transformer.transform(message).await.unwrap();

        assert_eq!(chunks.len(), 1);
        assert_openai_error(&chunks[0], "rate_limit_error", "too many buffered requests");
    }

    #[tokio::test]
    async fn streaming_image_decoding_failed_returns_error_variant() {
        let transformer = streaming_transformer(false);

        let message = token_message(GeneratedTokenResult::ImageDecodingFailed(
            "unsupported format".to_owned(),
        ));
        let chunks = transformer.transform(message).await.unwrap();

        assert_eq!(chunks.len(), 1);
        assert_openai_error(&chunks[0], "server_error", "unsupported format");
    }

    #[tokio::test]
    async fn streaming_multimodal_not_supported_returns_error_variant() {
        let transformer = streaming_transformer(false);

        let message = token_message(GeneratedTokenResult::MultimodalNotSupported(
            "model does not support images".to_owned(),
        ));
        let chunks = transformer.transform(message).await.unwrap();

        assert_eq!(chunks.len(), 1);
        assert_openai_error(&chunks[0], "server_error", "model does not support images");
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
        let chunks = transformer.transform(message).await.unwrap();

        assert_eq!(chunks.len(), 1);
        assert_openai_error(
            &chunks[0],
            "server_error",
            "image required 368 tokens but agent n_batch is 100; rerun with a larger n_batch",
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

        assert_eq!(chunks.len(), 1);
        assert_chunk_contains(&chunks[0], "{not valid json");
        assert_chunk_contains(&chunks[0], "\"name\":\"broken_tool\"");
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
        assert_openai_error(
            &chunks[0],
            "invalid_request_error",
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
        assert_openai_error(
            &chunks[0],
            "server_error",
            "grammar conflicts with thinking",
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

        assert_eq!(chunks.len(), 1);
        assert_openai_error(&chunks[0], "server_error", "output rejected by grammar");
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
        assert_openai_error(&chunks[0], "server_error", "could not build grammar");
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
        assert_openai_error(&chunks[0], "server_error", "bad grammar syntax");
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
        assert_openai_error(&chunks[0], "server_error", "sampler blew up");
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
        assert_openai_error(&chunks[0], "server_error", "schema is not valid");
    }
}
