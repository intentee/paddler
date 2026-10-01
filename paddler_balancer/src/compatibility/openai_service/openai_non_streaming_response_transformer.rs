use std::mem::take;
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
use crate::compatibility::openai_service::chat_completion::ChatCompletion;
use crate::compatibility::openai_service::chat_completion_finish_reason::chat_completion_finish_reason;
use crate::compatibility::openai_service::openai_non_streaming_state::OpenAINonStreamingState;
use crate::compatibility::openai_service::openai_usage::OpenAIUsage;
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

    fn build_done_chunk(
        &self,
        request_id: &str,
        summary: &GenerationSummary,
    ) -> Result<String, AgentRelayError> {
        let snapshot = take(&mut *self.state.lock());

        let has_tool_calls = !snapshot.tool_calls.is_empty();
        let finish_reason = chat_completion_finish_reason(summary.finish, has_tool_calls);

        to_string(&ChatCompletion {
            content: &snapshot.content,
            created: self.created,
            finish_reason,
            id: request_id,
            model: &self.model,
            tool_calls: &snapshot.tool_calls,
            usage: OpenAIUsage(summary.usage),
        })
        .map_err(AgentRelayError::MessageUnserializable)
    }
}

#[async_trait]
impl TransformsOutgoingMessage for OpenAINonStreamingResponseTransformer {
    type Output = TransformResult;

    async fn transform(
        &self,
        message: OutgoingMessage,
    ) -> Result<Vec<TransformResult>, AgentRelayError> {
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

    use super::OpenAINonStreamingResponseTransformer;
    use super::OpenAINonStreamingState;
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

    fn non_streaming_transformer() -> OpenAINonStreamingResponseTransformer {
        OpenAINonStreamingResponseTransformer {
            created: 0,
            model: "test-model".to_owned(),
            state: Arc::new(Mutex::new(OpenAINonStreamingState::default())),
        }
    }

    #[tokio::test]
    async fn non_streaming_aggregates_content_only_when_no_reasoning() {
        let transformer = non_streaming_transformer();

        transformer
            .transform(token_message(GeneratedTokenResult::ContentToken(
                "hel".to_owned(),
            )))
            .await
            .expect("the transformer must accept the message");
        transformer
            .transform(token_message(GeneratedTokenResult::ContentToken(
                "lo".to_owned(),
            )))
            .await
            .expect("the transformer must accept the message");

        let summary = summary_with_counts(4, 2, 0);
        let final_chunks = transformer
            .transform(token_message(GeneratedTokenResult::Done(summary)))
            .await
            .expect("the transformer must accept the message");

        assert_eq!(
            final_chunks,
            vec![
                TransformResult::Chunk(r#"{"id":"test-request","object":"chat.completion","created":0,"model":"test-model","choices":[{"index":0,"message":{"role":"assistant","content":"hello","refusal":null,"annotations":[]},"logprobs":null,"finish_reason":"stop"}],"usage":{"prompt_tokens":4,"completion_tokens":2,"total_tokens":6,"prompt_tokens_details":{"cached_tokens":0,"audio_tokens":0},"completion_tokens_details":{"reasoning_tokens":0}},"service_tier":"default"}"#.to_owned()),
            ]
        );
    }

    #[tokio::test]
    async fn non_streaming_drops_reasoning_but_keeps_reasoning_token_count() {
        let transformer = non_streaming_transformer();

        transformer
            .transform(token_message(GeneratedTokenResult::ReasoningToken(
                "think".to_owned(),
            )))
            .await
            .expect("the transformer must accept the message");
        transformer
            .transform(token_message(GeneratedTokenResult::ContentToken(
                "answer".to_owned(),
            )))
            .await
            .expect("the transformer must accept the message");

        let summary = summary_with_counts(3, 1, 1);
        let final_chunks = transformer
            .transform(token_message(GeneratedTokenResult::Done(summary)))
            .await
            .expect("the transformer must accept the message");

        assert_eq!(
            final_chunks,
            vec![
                TransformResult::Chunk(r#"{"id":"test-request","object":"chat.completion","created":0,"model":"test-model","choices":[{"index":0,"message":{"role":"assistant","content":"answer","refusal":null,"annotations":[]},"logprobs":null,"finish_reason":"stop"}],"usage":{"prompt_tokens":3,"completion_tokens":2,"total_tokens":5,"prompt_tokens_details":{"cached_tokens":0,"audio_tokens":0},"completion_tokens_details":{"reasoning_tokens":1}},"service_tier":"default"}"#.to_owned()),
            ]
        );
    }

    #[tokio::test]
    async fn non_streaming_undeterminable_routes_to_content() {
        let transformer = non_streaming_transformer();

        transformer
            .transform(token_message(GeneratedTokenResult::UndeterminableToken(
                "amb".to_owned(),
            )))
            .await
            .expect("the transformer must accept the message");

        let summary = summary_with_counts(2, 0, 0);
        let final_chunks = transformer
            .transform(token_message(GeneratedTokenResult::Done(summary)))
            .await
            .expect("the transformer must accept the message");

        assert_eq!(
            final_chunks,
            vec![
                TransformResult::Chunk(r#"{"id":"test-request","object":"chat.completion","created":0,"model":"test-model","choices":[{"index":0,"message":{"role":"assistant","content":"amb","refusal":null,"annotations":[]},"logprobs":null,"finish_reason":"stop"}],"usage":{"prompt_tokens":2,"completion_tokens":0,"total_tokens":2,"prompt_tokens_details":{"cached_tokens":0,"audio_tokens":0},"completion_tokens_details":{"reasoning_tokens":0}},"service_tier":"default"}"#.to_owned()),
            ]
        );
    }

    #[tokio::test]
    async fn non_streaming_tool_call_parsed_populates_message_tool_calls() {
        let transformer = non_streaming_transformer();

        transformer
            .transform(token_message(GeneratedTokenResult::ToolCallParsed(vec![
                weather_call(),
            ])))
            .await
            .expect("the transformer must accept the message");

        let summary = summary_with_counts(4, 0, 0);
        let final_chunks = transformer
            .transform(token_message(GeneratedTokenResult::Done(summary)))
            .await
            .expect("the transformer must accept the message");

        assert_eq!(
            final_chunks,
            vec![
                TransformResult::Chunk(r#"{"id":"test-request","object":"chat.completion","created":0,"model":"test-model","choices":[{"index":0,"message":{"role":"assistant","content":null,"refusal":null,"annotations":[],"tool_calls":[{"id":"call_x","type":"function","function":{"name":"get_weather","arguments":"{\"location\":\"Paris\"}"}}]},"logprobs":null,"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":4,"completion_tokens":0,"total_tokens":4,"prompt_tokens_details":{"cached_tokens":0,"audio_tokens":0},"completion_tokens_details":{"reasoning_tokens":0}},"service_tier":"default"}"#.to_owned()),
            ]
        );
    }

    #[tokio::test]
    async fn non_streaming_tool_call_parse_failed_emits_server_error() {
        let transformer = non_streaming_transformer();

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
    async fn non_streaming_tool_call_validation_failed_emits_server_error() {
        let transformer = non_streaming_transformer();

        let chunks = transformer
            .transform(token_message(
                GeneratedTokenResult::ToolCallValidationFailed(vec!["bad shape".to_owned()]),
            ))
            .await
            .expect("the transformer must accept the message");

        assert_eq!(
            chunks,
            vec![
                TransformResult::Error(r#"{"error":{"message":"bad shape","type":"server_error","param":null,"code":null}}"#.to_owned()),
            ]
        );
    }

    #[tokio::test]
    async fn non_streaming_unrecognized_tool_call_format_emits_server_error() {
        let transformer = non_streaming_transformer();

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
    async fn non_streaming_error_message_returns_server_error() {
        let transformer = non_streaming_transformer();

        let chunks = transformer
            .transform(error_message(500, "internal server error"))
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
    async fn non_streaming_chat_template_error_returns_server_error() {
        let transformer = non_streaming_transformer();

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
    async fn non_streaming_image_decoding_failed_returns_invalid_request_error() {
        let transformer = non_streaming_transformer();

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
    async fn non_streaming_multimodal_not_supported_returns_invalid_request_error() {
        let transformer = non_streaming_transformer();

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
    async fn non_streaming_media_exceeding_the_micro_batch_returns_invalid_request_error() {
        let transformer = non_streaming_transformer();

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

        assert_eq!(
            final_chunks,
            vec![
                TransformResult::Chunk(r#"{"id":"test-request","object":"chat.completion","created":0,"model":"test-model","choices":[{"index":0,"message":{"role":"assistant","content":null,"refusal":null,"annotations":[],"tool_calls":[{"id":"call_invalid","type":"function","function":{"name":"broken_tool","arguments":"{not valid json"}}]},"logprobs":null,"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":3,"completion_tokens":0,"total_tokens":3,"prompt_tokens_details":{"cached_tokens":0,"audio_tokens":0},"completion_tokens_details":{"reasoning_tokens":0}},"service_tier":"default"}"#.to_owned()),
            ]
        );
    }

    #[tokio::test]
    async fn non_streaming_embedding_response_returns_invalid_request_error() {
        let transformer = non_streaming_transformer();

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
    async fn rejects_inference_socket_notifications() {
        let transform_result = non_streaming_transformer()
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
