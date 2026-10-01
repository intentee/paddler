use std::mem::take;
use std::sync::Arc;

use async_trait::async_trait;
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
use crate::compatibility::openai_service::responses_non_streaming_state::ResponsesNonStreamingState;
use crate::compatibility::openai_service::responses_output_item::ResponsesOutputItem;
use crate::compatibility::openai_service::responses_output_item_kind::ResponsesOutputItemKind;
use crate::compatibility::openai_service::responses_response_header::ResponsesResponseHeader;
use crate::compatibility::openai_service::try_universal_error_chunk::try_universal_error_chunk;

#[derive(Clone)]
pub struct ResponsesNonStreamingResponseTransformer {
    pub header: ResponsesResponseHeader,
    pub state: Arc<Mutex<ResponsesNonStreamingState>>,
}

impl ResponsesNonStreamingResponseTransformer {
    fn build_completed(&self, summary: &GenerationSummary) -> Result<String, AgentRelayError> {
        let snapshot = take(&mut *self.state.lock());

        let mut output: Vec<ResponsesOutputItem> = Vec::new();

        if !snapshot.reasoning.is_empty() {
            output.push(ResponsesOutputItem::completed_reasoning(
                ResponsesOutputItemKind::Reasoning.item_id(output.len()),
                snapshot.reasoning,
            ));
        }

        let has_tool_calls = !snapshot.tool_calls.is_empty();

        if !snapshot.content.is_empty() || !has_tool_calls {
            output.push(ResponsesOutputItem::completed_message(
                ResponsesOutputItemKind::Message.item_id(output.len()),
                snapshot.content,
            ));
        }

        for call in &snapshot.tool_calls {
            output.push(ResponsesOutputItem::completed_function_call(
                ResponsesOutputItemKind::FunctionCall.item_id(output.len()),
                call,
            ));
        }

        to_string(&self.header.finished(output, &summary.usage, summary.finish))
            .map_err(AgentRelayError::MessageUnserializable)
    }
}

#[async_trait]
impl TransformsOutgoingMessage for ResponsesNonStreamingResponseTransformer {
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
                self.state.lock().content.push_str(&text);
                Ok(vec![])
            }
            OutgoingMessage::Response(ResponseEnvelope {
                response:
                    OutgoingResponse::GeneratedToken(GeneratedTokenResult::ReasoningToken(text)),
                ..
            }) => {
                self.state.lock().reasoning.push_str(&text);
                Ok(vec![])
            }
            OutgoingMessage::Response(ResponseEnvelope {
                response: OutgoingResponse::GeneratedToken(GeneratedTokenResult::ToolCallToken(_)),
                ..
            }) => Ok(vec![]),
            OutgoingMessage::Response(ResponseEnvelope {
                response:
                    OutgoingResponse::GeneratedToken(GeneratedTokenResult::ToolCallParsed(parsed_calls)),
                ..
            }) => {
                self.state.lock().tool_calls.extend(parsed_calls);
                Ok(vec![])
            }
            OutgoingMessage::Response(ResponseEnvelope {
                response: OutgoingResponse::GeneratedToken(GeneratedTokenResult::Done(summary)),
                ..
            }) => Ok(vec![TransformResult::Chunk(
                self.build_completed(&summary)?,
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
    use std::mem::discriminant;
    use std::sync::Arc;

    use llama_cpp_bindings_types::ParsedToolCall;
    use llama_cpp_bindings_types::TokenUsage;
    use llama_cpp_bindings_types::ToolCallArguments;
    use parking_lot::Mutex;
    use serde_json::Value;
    use serde_json::from_slice;
    use serde_json::json;

    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use paddler_messaging::generation_finish::GenerationFinish;
    use paddler_messaging::generation_summary::GenerationSummary;
    use paddler_messaging::inference_client::message::Message as OutgoingMessage;
    use paddler_messaging::inference_client::notification::Notification;
    use paddler_messaging::inference_client::response::Response as OutgoingResponse;
    use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
    use paddler_openai_response_format_validator::openai_validator::OpenAIValidator;

    use super::ResponsesNonStreamingResponseTransformer;
    use super::ResponsesNonStreamingState;
    use crate::agent_relay_error::AgentRelayError;
    use crate::chunk_forwarding_session_controller::transform_result::TransformResult;
    use crate::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;
    use crate::compatibility::openai_service::responses_response_header::ResponsesResponseHeader;

    #[must_use]
    pub fn token_message(token_result: GeneratedTokenResult) -> OutgoingMessage {
        OutgoingMessage::Response(ResponseEnvelope {
            generated_by: None,
            request_id: "test-request".to_owned(),
            response: OutgoingResponse::GeneratedToken(token_result),
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
    pub fn header() -> ResponsesResponseHeader {
        ResponsesResponseHeader {
            id: "resp_test".to_owned(),
            created_at: 0,
            model: "test-model".to_owned(),
            instructions: None,
            temperature: 0.25,
            top_p: 0.5,
        }
    }

    fn only_chunk(chunks: Vec<TransformResult>) -> Value {
        let [chunk] = <[TransformResult; 1]>::try_from(chunks).unwrap();

        assert_eq!(
            discriminant(&chunk),
            discriminant(&TransformResult::Chunk(String::new()))
        );

        from_slice(&chunk.into_ndjson_line().unwrap()).unwrap()
    }

    fn non_streaming_transformer() -> ResponsesNonStreamingResponseTransformer {
        ResponsesNonStreamingResponseTransformer {
            header: header(),
            state: Arc::new(Mutex::new(ResponsesNonStreamingState::default())),
        }
    }

    #[tokio::test]
    async fn non_streaming_aggregates_content_into_a_message_item() {
        let transformer = non_streaming_transformer();

        transformer
            .transform(token_message(GeneratedTokenResult::ContentToken(
                "hel".to_owned(),
            )))
            .await
            .unwrap();
        transformer
            .transform(token_message(GeneratedTokenResult::ContentToken(
                "lo".to_owned(),
            )))
            .await
            .unwrap();
        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::Done(
                summary_with_counts(3, 2, 0),
            )))
            .await
            .unwrap();

        let response = only_chunk(chunks);

        assert_eq!(response["object"], "response");
        assert_eq!(response["status"], "completed");
        assert_eq!(response["output"][0]["type"], "message");
        assert_eq!(response["output"][0]["content"][0]["text"], "hello");
    }

    #[tokio::test]
    async fn non_streaming_surfaces_reasoning_and_tool_calls_in_output() {
        let transformer = non_streaming_transformer();

        transformer
            .transform(token_message(GeneratedTokenResult::ReasoningToken(
                "ponder".to_owned(),
            )))
            .await
            .unwrap();
        transformer
            .transform(token_message(GeneratedTokenResult::ToolCallParsed(vec![
                weather_call(),
            ])))
            .await
            .unwrap();
        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::Done(
                summary_with_counts(3, 0, 1),
            )))
            .await
            .unwrap();

        let response = only_chunk(chunks);

        assert_eq!(response["output"][0]["type"], "reasoning");
        assert_eq!(response["output"][1]["type"], "function_call");
        assert_eq!(response["output"][1]["name"], "get_weather");
        assert_eq!(
            response["usage"]["output_tokens_details"]["reasoning_tokens"],
            1
        );
    }

    #[tokio::test]
    async fn non_streaming_error_returns_an_error_envelope() {
        let transformer = non_streaming_transformer();

        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::SamplerError(
                "sampler blew up".to_owned(),
            )))
            .await
            .unwrap();

        assert_eq!(
            chunks,
            vec![TransformResult::Error(
                r#"{"error":{"message":"sampler blew up","type":"server_error","param":null,"code":null}}"#
                    .to_owned()
            )]
        );
    }

    #[tokio::test]
    async fn the_non_streaming_response_conforms_to_the_schema() {
        let validator = OpenAIValidator::new().unwrap();
        let transformer = non_streaming_transformer();

        transformer
            .transform(token_message(GeneratedTokenResult::ReasoningToken(
                "p".to_owned(),
            )))
            .await
            .unwrap();
        transformer
            .transform(token_message(GeneratedTokenResult::ContentToken(
                "hello".to_owned(),
            )))
            .await
            .unwrap();
        transformer
            .transform(token_message(GeneratedTokenResult::ToolCallParsed(vec![
                weather_call(),
            ])))
            .await
            .unwrap();
        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::Done(
                summary_with_counts(5, 3, 2),
            )))
            .await
            .unwrap();

        let response = only_chunk(chunks);

        validator.validate_responses_response(&response).unwrap();
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
