use std::sync::Arc;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
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
    fn content_chunk(&self, request_id: &str, text: &str) -> Result<String> {
        serde_json::to_string(&json!({
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
        }))
        .context("serializing content chunk")
    }

    fn tool_calls_chunk(
        &self,
        request_id: &str,
        parsed_calls: &[ParsedToolCall],
    ) -> Result<String> {
        parsed_calls
            .iter()
            .enumerate()
            .map(|(index, call)| {
                arguments_to_tool_call_string(&call.arguments).map(|arguments| {
                    json!({
                        "index": index,
                        "id": call.id,
                        "type": "function",
                        "function": {
                            "name": call.name,
                            "arguments": arguments,
                        }
                    })
                })
            })
            .collect::<Result<Vec<_>>>()
            .and_then(|tool_calls| {
                serde_json::to_string(&json!({
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
                }))
                .context("serializing tool-calls chunk")
            })
    }

    fn finish_chunk(&self, request_id: &str, finish_reason: &str) -> Result<String> {
        serde_json::to_string(&json!({
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
        }))
        .context("serializing finish chunk")
    }

    fn usage_chunk(&self, request_id: &str, usage: &TokenUsage) -> Result<String> {
        serde_json::to_string(&json!({
            "id": request_id,
            "object": "chat.completion.chunk",
            "created": self.created,
            "model": self.model,
            "system_fingerprint": self.system_fingerprint,
            "choices": [],
            "usage": openai_usage_json(usage),
        }))
        .context("serializing usage chunk")
    }

    fn handle_content(&self, request_id: &str, text: &str) -> Result<Vec<TransformResult>> {
        self.content_chunk(request_id, text)
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

        self.tool_calls_chunk(request_id, parsed_calls)
            .map(|chunk| vec![TransformResult::Chunk(chunk)])
    }

    fn handle_done(
        &self,
        request_id: &str,
        summary: &GenerationSummary,
    ) -> Result<Vec<TransformResult>> {
        let saw_tool_call = self.state.lock().saw_tool_call;
        let finish_reason = if saw_tool_call { "tool_calls" } else { "stop" };

        self.finish_chunk(request_id, finish_reason)
            .and_then(|finish_chunk| {
                let finish = TransformResult::Chunk(finish_chunk);

                if self.include_usage {
                    self.usage_chunk(request_id, &summary.usage)
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
        if let Some(error_chunk) = try_universal_error_chunk(&message) {
            return Ok(vec![error_chunk]);
        }

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
            other => Err(anyhow!(
                "OpenAIStreamingResponseTransformer received an outgoing message it does not know how to handle: {other:?}"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use anyhow::Result;
    use llama_cpp_bindings_types::ParsedToolCall;
    use llama_cpp_bindings_types::TokenUsage;
    use llama_cpp_bindings_types::ToolCallArguments;
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

    pub fn assert_chunk_contains(result: &TransformResult, expected: &str) -> Result<()> {
        let TransformResult::Chunk(content) = result else {
            anyhow::bail!("expected TransformResult::Chunk, got TransformResult::Error");
        };

        assert!(
            content.contains(expected),
            "chunk does not contain '{expected}': {content}"
        );

        Ok(())
    }

    pub fn assert_chunk_does_not_contain(result: &TransformResult, expected: &str) -> Result<()> {
        let TransformResult::Chunk(content) = result else {
            anyhow::bail!("expected TransformResult::Chunk, got TransformResult::Error");
        };

        assert!(
            !content.contains(expected),
            "chunk unexpectedly contains '{expected}': {content}"
        );

        Ok(())
    }

    pub fn assert_error_contains(result: &TransformResult, expected: &str) -> Result<()> {
        let TransformResult::Error(content) = result else {
            anyhow::bail!("expected TransformResult::Error, got TransformResult::Chunk");
        };

        assert!(
            content.contains(expected),
            "error does not contain '{expected}': {content}"
        );

        Ok(())
    }

    pub fn assert_chunk_body_contains(result: &TransformResult, expected: &str) {
        let TransformResult::Chunk(content) = result else {
            panic!("expected a chunk variant");
        };

        assert!(
            content.contains(expected),
            "chunk does not contain '{expected}': {content}"
        );
    }

    pub fn assert_error_body_contains(result: &TransformResult, expected: &str) {
        let TransformResult::Error(content) = result else {
            panic!("expected an error variant");
        };

        assert!(
            content.contains(expected),
            "error does not contain '{expected}': {content}"
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
    async fn streaming_content_token_emits_content_delta() -> Result<()> {
        let transformer = streaming_transformer(false);

        let message = token_message(GeneratedTokenResult::ContentToken("hello".to_owned()));
        let chunks = transformer.transform(message).await?;

        assert_eq!(chunks.len(), 1);
        assert_chunk_contains(&chunks[0], "\"content\":\"hello\"")?;
        assert_chunk_contains(&chunks[0], "\"role\":\"assistant\"")?;
        assert_chunk_does_not_contain(&chunks[0], "reasoning_content")?;

        Ok(())
    }

    #[tokio::test]
    async fn streaming_reasoning_token_is_dropped() -> Result<()> {
        let transformer = streaming_transformer(false);

        let message = token_message(GeneratedTokenResult::ReasoningToken("thought".to_owned()));
        let chunks = transformer.transform(message).await?;

        assert_eq!(chunks.len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn streaming_undeterminable_token_emits_content_delta() -> Result<()> {
        let transformer = streaming_transformer(false);

        let message = token_message(GeneratedTokenResult::UndeterminableToken(
            "ambig".to_owned(),
        ));
        let chunks = transformer.transform(message).await?;

        assert_eq!(chunks.len(), 1);
        assert_chunk_contains(&chunks[0], "\"content\":\"ambig\"")?;
        assert_chunk_does_not_contain(&chunks[0], "reasoning_content")?;

        Ok(())
    }

    #[tokio::test]
    async fn streaming_tool_call_token_is_silently_dropped() -> Result<()> {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::ToolCallToken(
                "{".to_owned(),
            )))
            .await?;

        assert_eq!(chunks.len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn streaming_tool_call_parsed_emits_structured_tool_calls_chunk() -> Result<()> {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::ToolCallParsed(vec![
                weather_call(),
            ])))
            .await?;

        assert_eq!(chunks.len(), 1);
        assert_chunk_contains(&chunks[0], "\"tool_calls\"")?;
        assert_chunk_contains(&chunks[0], "\"id\":\"call_x\"")?;
        assert_chunk_contains(&chunks[0], "\"name\":\"get_weather\"")?;
        assert_chunk_contains(
            &chunks[0],
            "\"arguments\":\"{\\\"location\\\":\\\"Paris\\\"}\"",
        )?;

        Ok(())
    }

    #[tokio::test]
    async fn streaming_done_after_tool_call_uses_tool_calls_finish_reason() -> Result<()> {
        let transformer = streaming_transformer(false);

        transformer
            .transform(token_message(GeneratedTokenResult::ToolCallParsed(vec![
                weather_call(),
            ])))
            .await?;

        let summary = summary_with_counts(2, 0, 0);
        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::Done(summary)))
            .await?;

        assert_eq!(chunks.len(), 1);
        assert_chunk_contains(&chunks[0], "\"finish_reason\":\"tool_calls\"")?;

        Ok(())
    }

    #[tokio::test]
    async fn streaming_done_without_tool_call_uses_stop_finish_reason() -> Result<()> {
        let transformer = streaming_transformer(false);

        transformer
            .transform(token_message(GeneratedTokenResult::ContentToken(
                "hi".to_owned(),
            )))
            .await?;

        let summary = summary_with_counts(2, 1, 0);
        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::Done(summary)))
            .await?;

        assert_eq!(chunks.len(), 1);
        assert_chunk_contains(&chunks[0], "\"finish_reason\":\"stop\"")?;

        Ok(())
    }

    #[tokio::test]
    async fn streaming_done_with_include_usage_emits_finish_then_usage_chunk() -> Result<()> {
        let transformer = streaming_transformer(true);
        let summary = summary_with_counts(7, 4, 1);

        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::Done(summary)))
            .await?;

        assert_eq!(chunks.len(), 2);
        assert_chunk_contains(&chunks[0], "\"finish_reason\":\"stop\"")?;
        assert_chunk_does_not_contain(&chunks[0], "usage")?;
        assert_chunk_contains(&chunks[1], "\"prompt_tokens\":7")?;
        assert_chunk_contains(&chunks[1], "\"completion_tokens\":5")?;
        assert_chunk_contains(&chunks[1], "\"total_tokens\":12")?;
        assert_chunk_contains(&chunks[1], "\"choices\":[]")?;

        Ok(())
    }

    #[tokio::test]
    async fn streaming_done_without_include_usage_emits_only_finish_chunk() -> Result<()> {
        let transformer = streaming_transformer(false);
        let summary = summary_with_counts(5, 3, 2);

        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::Done(summary)))
            .await?;

        assert_eq!(chunks.len(), 1);
        assert_chunk_contains(&chunks[0], "\"finish_reason\":\"stop\"")?;
        assert_chunk_does_not_contain(&chunks[0], "usage")?;

        Ok(())
    }

    #[tokio::test]
    async fn streaming_tool_call_parse_failed_emits_server_error() -> Result<()> {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::ToolCallParseFailed(
                "bad payload".to_owned(),
            )))
            .await?;

        assert_eq!(chunks.len(), 1);
        assert_error_contains(&chunks[0], "bad payload")?;
        assert_error_contains(&chunks[0], "server_error")?;

        Ok(())
    }

    #[tokio::test]
    async fn streaming_tool_call_validation_failed_emits_server_error() -> Result<()> {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(token_message(
                GeneratedTokenResult::ToolCallValidationFailed(vec!["missing field x".to_owned()]),
            ))
            .await?;

        assert_eq!(chunks.len(), 1);
        assert_error_contains(&chunks[0], "missing field x")?;

        Ok(())
    }

    #[tokio::test]
    async fn streaming_unrecognized_tool_call_format_emits_server_error() -> Result<()> {
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
            .await?;

        assert_eq!(chunks.len(), 1);
        assert_error_contains(&chunks[0], "common_chat_parse failed: no parser")?;
        assert_error_contains(&chunks[0], "<unknown_marker>blah</unknown_marker>")?;
        assert_error_contains(&chunks[0], "server_error")?;

        Ok(())
    }

    #[tokio::test]
    async fn streaming_error_message_returns_error_variant() -> Result<()> {
        let transformer = streaming_transformer(false);

        let message = error_message(500, "internal server error");
        let chunks = transformer.transform(message).await?;

        assert_eq!(chunks.len(), 1);
        assert_error_contains(&chunks[0], "internal server error")?;
        assert_error_contains(&chunks[0], "server_error")?;

        Ok(())
    }

    #[tokio::test]
    async fn streaming_chat_template_error_returns_error_variant() -> Result<()> {
        let transformer = streaming_transformer(false);

        let message = token_message(GeneratedTokenResult::ChatTemplateError(
            "bad template".to_owned(),
        ));
        let chunks = transformer.transform(message).await?;

        assert_eq!(chunks.len(), 1);
        assert_error_contains(&chunks[0], "bad template")?;
        assert_error_contains(&chunks[0], "server_error")?;

        Ok(())
    }

    #[tokio::test]
    async fn streaming_timeout_returns_error_variant() -> Result<()> {
        let transformer = streaming_transformer(false);

        let message = response_message(OutgoingResponse::Timeout);
        let chunks = transformer.transform(message).await?;

        assert_eq!(chunks.len(), 1);
        assert_error_contains(&chunks[0], "request timed out")?;
        assert_error_contains(&chunks[0], "timeout")?;

        Ok(())
    }

    #[tokio::test]
    async fn streaming_too_many_buffered_requests_returns_error_variant() -> Result<()> {
        let transformer = streaming_transformer(false);

        let message = response_message(OutgoingResponse::TooManyBufferedRequests);
        let chunks = transformer.transform(message).await?;

        assert_eq!(chunks.len(), 1);
        assert_error_contains(&chunks[0], "too many buffered requests")?;
        assert_error_contains(&chunks[0], "rate_limit_error")?;

        Ok(())
    }

    #[tokio::test]
    async fn streaming_image_decoding_failed_returns_error_variant() -> Result<()> {
        let transformer = streaming_transformer(false);

        let message = token_message(GeneratedTokenResult::ImageDecodingFailed(
            "unsupported format".to_owned(),
        ));
        let chunks = transformer.transform(message).await?;

        assert_eq!(chunks.len(), 1);
        assert_error_contains(&chunks[0], "unsupported format")?;
        assert_error_contains(&chunks[0], "server_error")?;

        Ok(())
    }

    #[tokio::test]
    async fn streaming_multimodal_not_supported_returns_error_variant() -> Result<()> {
        let transformer = streaming_transformer(false);

        let message = token_message(GeneratedTokenResult::MultimodalNotSupported(
            "model does not support images".to_owned(),
        ));
        let chunks = transformer.transform(message).await?;

        assert_eq!(chunks.len(), 1);
        assert_error_contains(&chunks[0], "model does not support images")?;
        assert_error_contains(&chunks[0], "server_error")?;

        Ok(())
    }

    #[tokio::test]
    async fn streaming_image_exceeds_batch_size_returns_error_variant() -> Result<()> {
        let transformer = streaming_transformer(false);

        let message = token_message(GeneratedTokenResult::ImageExceedsBatchSize(
            paddler_messaging::oversized_image_details::OversizedImageDetails {
                image_tokens: 368,
                n_batch: 100,
            },
        ));
        let chunks = transformer.transform(message).await?;

        assert_eq!(chunks.len(), 1);
        assert_error_contains(&chunks[0], "368")?;
        assert_error_contains(&chunks[0], "100")?;
        assert_error_contains(&chunks[0], "server_error")?;

        Ok(())
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
}
