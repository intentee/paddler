use std::sync::Arc;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use async_trait::async_trait;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::generation_summary::GenerationSummary;
use paddler_messaging::inference_client::message::Message as OutgoingMessage;
use paddler_messaging::inference_client::response::Response as OutgoingResponse;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use parking_lot::Mutex;
use serde_json::Value;

use crate::chunk_forwarding_session_controller::transform_result::TransformResult;
use crate::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;
use crate::compatibility::openai_service::arguments_to_tool_call_string::arguments_to_tool_call_string;
use crate::compatibility::openai_service::function_call_item::function_call_item;
use crate::compatibility::openai_service::message_item_done::message_item_done;
use crate::compatibility::openai_service::reasoning_item_done::reasoning_item_done;
use crate::compatibility::openai_service::responses_error::responses_error;
use crate::compatibility::openai_service::responses_non_streaming_state::ResponsesNonStreamingState;
use crate::compatibility::openai_service::responses_response_builder::ResponsesResponseBuilder;

#[derive(Clone)]
pub struct ResponsesNonStreamingResponseTransformer {
    pub builder: ResponsesResponseBuilder,
    pub state: Arc<Mutex<ResponsesNonStreamingState>>,
}

impl ResponsesNonStreamingResponseTransformer {
    fn build_completed(&self, summary: &GenerationSummary) -> Result<String> {
        let snapshot = self.state.lock().clone();

        let mut output: Vec<Value> = Vec::new();

        if !snapshot.reasoning.is_empty() {
            output.push(reasoning_item_done(
                &format!("rs_{}", output.len()),
                &snapshot.reasoning,
            ));
        }

        let has_tool_calls = !snapshot.tool_calls.is_empty();

        if !snapshot.content.is_empty() || !has_tool_calls {
            output.push(message_item_done(
                &format!("msg_{}", output.len()),
                &snapshot.content,
            ));
        }

        for call in &snapshot.tool_calls {
            let arguments = arguments_to_tool_call_string(&call.arguments)?;

            output.push(function_call_item(
                &format!("fc_{}", output.len()),
                &call.id,
                &call.name,
                &arguments,
                "completed",
            ));
        }

        serde_json::to_string(&self.builder.completed(output, &summary.usage))
            .context("serializing non-streaming responses completion")
    }
}

#[async_trait]
impl TransformsOutgoingMessage for ResponsesNonStreamingResponseTransformer {
    type Output = TransformResult;

    async fn transform(&self, message: OutgoingMessage) -> Result<Vec<TransformResult>> {
        if let Some(error) = responses_error(&message) {
            return Ok(vec![TransformResult::Error(
                error.to_envelope().to_string(),
            )]);
        }

        match message {
            OutgoingMessage::Response(ResponseEnvelope {
                response: OutgoingResponse::GeneratedToken(token),
                ..
            }) => match token {
                GeneratedTokenResult::ContentToken(text)
                | GeneratedTokenResult::UndeterminableToken(text) => {
                    self.state.lock().content.push_str(&text);
                    Ok(vec![])
                }
                GeneratedTokenResult::ReasoningToken(text) => {
                    self.state.lock().reasoning.push_str(&text);
                    Ok(vec![])
                }
                GeneratedTokenResult::ToolCallToken(_) => Ok(vec![]),
                GeneratedTokenResult::ToolCallParsed(parsed_calls) => {
                    self.state.lock().tool_calls.extend(parsed_calls);
                    Ok(vec![])
                }
                GeneratedTokenResult::Done(summary) => Ok(vec![TransformResult::Chunk(
                    self.build_completed(&summary)?,
                )]),
                other => Err(anyhow!(
                    "ResponsesNonStreamingResponseTransformer received a token it does not know how to handle: {other:?}"
                )),
            },
            other => Err(anyhow!(
                "ResponsesNonStreamingResponseTransformer received an outgoing message it does not know how to handle: {other:?}"
            )),
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
    use paddler_messaging::inference_client::response::Response as OutgoingResponse;
    use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
    use paddler_openai_response_format_validator::openai_validator::OpenAIValidator;
    use parking_lot::Mutex;
    use serde_json::json;

    use crate::chunk_forwarding_session_controller::transform_result::TransformResult;
    use crate::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;
    use crate::compatibility::openai_service::responses_response_builder::ResponsesResponseBuilder;

    use super::ResponsesNonStreamingResponseTransformer;
    use super::ResponsesNonStreamingState;

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
    pub fn builder() -> ResponsesResponseBuilder {
        ResponsesResponseBuilder {
            id: "resp_test".to_owned(),
            created_at: 0,
            model: "test-model".to_owned(),
            instructions: None,
        }
    }

    fn non_streaming_transformer() -> ResponsesNonStreamingResponseTransformer {
        ResponsesNonStreamingResponseTransformer {
            builder: builder(),
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

        let TransformResult::Chunk(body) = &chunks[0] else {
            panic!("expected a chunk");
        };
        let response: serde_json::Value = serde_json::from_str(body).unwrap();

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

        let TransformResult::Chunk(body) = &chunks[0] else {
            panic!("expected a chunk");
        };
        let response: serde_json::Value = serde_json::from_str(body).unwrap();

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

        let TransformResult::Error(body) = &chunks[0] else {
            panic!("expected an error");
        };

        assert!(body.contains("sampler blew up"));
        assert!(body.contains("server_error"));
    }

    #[tokio::test]
    async fn the_non_streaming_response_conforms_to_the_official_schema() {
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

        let TransformResult::Chunk(body) = &chunks[0] else {
            panic!("expected a chunk");
        };
        let response: serde_json::Value = serde_json::from_str(body).unwrap();

        validator.validate_responses_response(&response).unwrap();
    }
}
