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
use serde_json::json;

use crate::chunk_forwarding_session_controller::transform_result::TransformResult;
use crate::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;
use crate::compatibility::openai_service::arguments_to_tool_call_string::arguments_to_tool_call_string;
use crate::compatibility::openai_service::openai_non_streaming_state::OpenAINonStreamingState;
use crate::compatibility::openai_service::openai_usage_json::openai_usage_json;
use crate::compatibility::openai_service::try_universal_error_chunk::try_universal_error_chunk;

#[derive(Clone)]
pub struct OpenAINonStreamingResponseTransformer {
    pub created: u64,
    pub model: String,
    pub state: Arc<Mutex<OpenAINonStreamingState>>,
}

impl OpenAINonStreamingResponseTransformer {
    fn append_content(&self, text: &str) {
        self.state.lock().content.push_str(text);
    }

    fn append_tool_calls(&self, parsed_calls: Vec<ParsedToolCall>) {
        self.state.lock().tool_calls.extend(parsed_calls);
    }

    fn build_done_chunk(&self, request_id: &str, summary: &GenerationSummary) -> Result<String> {
        let snapshot = self.snapshot_state();

        let has_tool_calls = !snapshot.tool_calls.is_empty();
        let finish_reason = if has_tool_calls { "tool_calls" } else { "stop" };

        let tool_calls_json = snapshot
            .tool_calls
            .iter()
            .map(|call| {
                arguments_to_tool_call_string(&call.arguments).map(|arguments| {
                    json!({
                        "id": call.id,
                        "type": "function",
                        "function": {
                            "name": call.name,
                            "arguments": arguments,
                        }
                    })
                })
            })
            .collect::<Result<Vec<_>>>();

        tool_calls_json.and_then(|tool_calls_json| {
            let mut message_obj = json!({
                "role": "assistant",
                "content": if snapshot.content.is_empty() && has_tool_calls {
                    serde_json::Value::Null
                } else {
                    json!(snapshot.content)
                },
                "refusal": null,
                "annotations": []
            });

            if has_tool_calls && let Some(map) = message_obj.as_object_mut() {
                map.insert("tool_calls".to_owned(), json!(tool_calls_json));
            }

            serde_json::to_string(&json!({
                "id": request_id,
                "object": "chat.completion",
                "created": self.created,
                "model": self.model,
                "choices": [
                    {
                        "index": 0,
                        "message": message_obj,
                        "logprobs": null,
                        "finish_reason": finish_reason
                    }
                ],
                "usage": openai_usage_json(&summary.usage),
                "service_tier": "default"
            }))
            .context("serializing non-streaming completion")
        })
    }

    fn snapshot_state(&self) -> OpenAINonStreamingState {
        self.state.lock().clone()
    }
}

#[async_trait]
impl TransformsOutgoingMessage for OpenAINonStreamingResponseTransformer {
    type Output = TransformResult;

    async fn transform(&self, message: OutgoingMessage) -> Result<Vec<TransformResult>> {
        if let Some(error_chunk) = try_universal_error_chunk(&message) {
            return Ok(vec![error_chunk]);
        }

        match message {
            OutgoingMessage::Response(ResponseEnvelope {
                response:
                    OutgoingResponse::GeneratedToken(
                        GeneratedTokenResult::ContentToken(text)
                        | GeneratedTokenResult::UndeterminableToken(text),
                    ),
                ..
            }) => {
                self.append_content(&text);
                Ok(vec![])
            }
            OutgoingMessage::Response(ResponseEnvelope {
                response:
                    OutgoingResponse::GeneratedToken(
                        GeneratedTokenResult::ReasoningToken(_)
                        | GeneratedTokenResult::ToolCallToken(_),
                    ),
                ..
            }) => Ok(vec![]),
            OutgoingMessage::Response(ResponseEnvelope {
                response:
                    OutgoingResponse::GeneratedToken(GeneratedTokenResult::ToolCallParsed(parsed_calls)),
                ..
            }) => {
                self.append_tool_calls(parsed_calls);
                Ok(vec![])
            }
            OutgoingMessage::Response(ResponseEnvelope {
                request_id,
                response: OutgoingResponse::GeneratedToken(GeneratedTokenResult::Done(summary)),
                ..
            }) => Ok(vec![TransformResult::Chunk(
                self.build_done_chunk(&request_id, &summary)?,
            )]),
            other => Err(anyhow!(
                "OpenAINonStreamingResponseTransformer received an outgoing message it does not know how to handle: {other:?}"
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

    use super::OpenAINonStreamingResponseTransformer;
    use super::OpenAINonStreamingState;
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

    fn non_streaming_transformer() -> OpenAINonStreamingResponseTransformer {
        OpenAINonStreamingResponseTransformer {
            created: 0,
            model: "test-model".to_owned(),
            state: Arc::new(Mutex::new(OpenAINonStreamingState::default())),
        }
    }

    #[tokio::test]
    async fn non_streaming_aggregates_content_only_when_no_reasoning() -> Result<()> {
        let transformer = non_streaming_transformer();

        transformer
            .transform(token_message(GeneratedTokenResult::ContentToken(
                "hel".to_owned(),
            )))
            .await?;
        transformer
            .transform(token_message(GeneratedTokenResult::ContentToken(
                "lo".to_owned(),
            )))
            .await?;

        let summary = summary_with_counts(4, 2, 0);
        let final_chunks = transformer
            .transform(token_message(GeneratedTokenResult::Done(summary)))
            .await?;

        assert_eq!(final_chunks.len(), 1);
        assert_chunk_contains(&final_chunks[0], "\"content\":\"hello\"")?;
        assert_chunk_does_not_contain(&final_chunks[0], "reasoning_content")?;
        assert_chunk_contains(&final_chunks[0], "\"prompt_tokens\":4")?;
        assert_chunk_contains(&final_chunks[0], "\"completion_tokens\":2")?;

        Ok(())
    }

    #[tokio::test]
    async fn non_streaming_drops_reasoning_but_keeps_reasoning_token_count() -> Result<()> {
        let transformer = non_streaming_transformer();

        transformer
            .transform(token_message(GeneratedTokenResult::ReasoningToken(
                "think".to_owned(),
            )))
            .await?;
        transformer
            .transform(token_message(GeneratedTokenResult::ContentToken(
                "answer".to_owned(),
            )))
            .await?;

        let summary = summary_with_counts(3, 1, 1);
        let final_chunks = transformer
            .transform(token_message(GeneratedTokenResult::Done(summary)))
            .await?;

        assert_eq!(final_chunks.len(), 1);
        assert_chunk_contains(&final_chunks[0], "\"content\":\"answer\"")?;
        assert_chunk_does_not_contain(&final_chunks[0], "reasoning_content")?;
        assert_chunk_contains(&final_chunks[0], "\"reasoning_tokens\":1")?;

        Ok(())
    }

    #[tokio::test]
    async fn non_streaming_undeterminable_routes_to_content() -> Result<()> {
        let transformer = non_streaming_transformer();

        transformer
            .transform(token_message(GeneratedTokenResult::UndeterminableToken(
                "amb".to_owned(),
            )))
            .await?;

        let summary = summary_with_counts(2, 0, 0);
        let final_chunks = transformer
            .transform(token_message(GeneratedTokenResult::Done(summary)))
            .await?;

        assert_eq!(final_chunks.len(), 1);
        assert_chunk_contains(&final_chunks[0], "\"content\":\"amb\"")?;
        assert_chunk_does_not_contain(&final_chunks[0], "reasoning_content")?;

        Ok(())
    }

    #[tokio::test]
    async fn non_streaming_tool_call_parsed_populates_message_tool_calls() -> Result<()> {
        let transformer = non_streaming_transformer();

        transformer
            .transform(token_message(GeneratedTokenResult::ToolCallParsed(vec![
                weather_call(),
            ])))
            .await?;

        let summary = summary_with_counts(4, 0, 0);
        let final_chunks = transformer
            .transform(token_message(GeneratedTokenResult::Done(summary)))
            .await?;

        assert_eq!(final_chunks.len(), 1);
        assert_chunk_contains(&final_chunks[0], "\"tool_calls\":")?;
        assert_chunk_contains(&final_chunks[0], "\"name\":\"get_weather\"")?;
        assert_chunk_contains(
            &final_chunks[0],
            "\"arguments\":\"{\\\"location\\\":\\\"Paris\\\"}\"",
        )?;
        assert_chunk_contains(&final_chunks[0], "\"finish_reason\":\"tool_calls\"")?;

        Ok(())
    }

    #[tokio::test]
    async fn non_streaming_tool_call_parse_failed_emits_error() -> Result<()> {
        let transformer = non_streaming_transformer();

        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::ToolCallParseFailed(
                "bad payload".to_owned(),
            )))
            .await?;

        assert_eq!(chunks.len(), 1);
        assert_error_contains(&chunks[0], "bad payload")?;

        Ok(())
    }

    #[tokio::test]
    async fn non_streaming_tool_call_validation_failed_emits_error() -> Result<()> {
        let transformer = non_streaming_transformer();

        let chunks = transformer
            .transform(token_message(
                GeneratedTokenResult::ToolCallValidationFailed(vec!["bad shape".to_owned()]),
            ))
            .await?;

        assert_eq!(chunks.len(), 1);
        assert_error_contains(&chunks[0], "bad shape")?;

        Ok(())
    }

    #[tokio::test]
    async fn non_streaming_unrecognized_tool_call_format_emits_server_error() -> Result<()> {
        let transformer = non_streaming_transformer();

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
    async fn non_streaming_error_message_returns_error_variant() -> Result<()> {
        let transformer = non_streaming_transformer();

        let chunks = transformer
            .transform(error_message(500, "internal server error"))
            .await?;

        assert_eq!(chunks.len(), 1);
        assert_error_contains(&chunks[0], "internal server error")?;
        assert_error_contains(&chunks[0], "server_error")?;

        Ok(())
    }

    #[tokio::test]
    async fn non_streaming_chat_template_error_returns_error_variant() -> Result<()> {
        let transformer = non_streaming_transformer();

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
    async fn non_streaming_image_decoding_failed_returns_error_variant() -> Result<()> {
        let transformer = non_streaming_transformer();

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
    async fn non_streaming_multimodal_not_supported_returns_error_variant() -> Result<()> {
        let transformer = non_streaming_transformer();

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
    async fn non_streaming_image_exceeds_batch_size_returns_error_variant() -> Result<()> {
        let transformer = non_streaming_transformer();

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
    async fn non_streaming_timeout_returns_error_variant() -> Result<()> {
        let transformer = non_streaming_transformer();

        let message = response_message(OutgoingResponse::Timeout);
        let chunks = transformer.transform(message).await?;

        assert_eq!(chunks.len(), 1);
        assert_error_contains(&chunks[0], "request timed out")?;
        assert_error_contains(&chunks[0], "timeout")?;

        Ok(())
    }

    #[tokio::test]
    async fn non_streaming_too_many_buffered_requests_returns_error_variant() -> Result<()> {
        let transformer = non_streaming_transformer();

        let message = response_message(OutgoingResponse::TooManyBufferedRequests);
        let chunks = transformer.transform(message).await?;

        assert_eq!(chunks.len(), 1);
        assert_error_contains(&chunks[0], "too many buffered requests")?;
        assert_error_contains(&chunks[0], "rate_limit_error")?;

        Ok(())
    }

    #[tokio::test]
    async fn non_streaming_tool_call_with_invalid_json_arguments_passes_raw_string_through() {
        let transformer = non_streaming_transformer();

        transformer
            .transform(token_message(GeneratedTokenResult::ToolCallParsed(vec![
                invalid_json_call(),
            ])))
            .await
            .unwrap();

        let final_chunks = transformer
            .transform(token_message(GeneratedTokenResult::Done(
                summary_with_counts(3, 0, 0),
            )))
            .await
            .unwrap();

        assert_eq!(final_chunks.len(), 1);
        assert_chunk_body_contains(&final_chunks[0], "{not valid json");
        assert_chunk_body_contains(&final_chunks[0], "\"name\":\"broken_tool\"");
    }

    #[tokio::test]
    async fn non_streaming_embedding_response_returns_invalid_request_error() {
        let transformer = non_streaming_transformer();

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
}
