use std::sync::Arc;

use anyhow::Result;
use anyhow::anyhow;
use async_trait::async_trait;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::generation_summary::GenerationSummary;
use paddler_messaging::inference_client::message::Message as OutgoingMessage;
use paddler_messaging::inference_client::response::Response as OutgoingResponse;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use parking_lot::Mutex;

use crate::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;
use crate::compatibility::openai_service::response_snapshot_event::ResponseSnapshotEvent;
use crate::compatibility::openai_service::responses_error::responses_error;
use crate::compatibility::openai_service::responses_response_builder::ResponsesResponseBuilder;
use crate::compatibility::openai_service::responses_stream_event::ResponsesStreamEvent;
use crate::compatibility::openai_service::responses_streaming_state::ResponsesStreamingState;

#[derive(Clone)]
pub struct ResponsesStreamingResponseTransformer {
    pub builder: ResponsesResponseBuilder,
    pub state: Arc<Mutex<ResponsesStreamingState>>,
}

impl ResponsesStreamingResponseTransformer {
    fn ensure_preamble(
        &self,
        state: &mut ResponsesStreamingState,
        events: &mut Vec<ResponsesStreamEvent>,
    ) {
        if state.started {
            return;
        }

        state.started = true;

        let created_sequence_number = state.next_sequence_number();
        events.push(ResponsesStreamEvent::Created(ResponseSnapshotEvent {
            sequence_number: created_sequence_number,
            response: self.builder.in_progress(),
        }));

        let in_progress_sequence_number = state.next_sequence_number();
        events.push(ResponsesStreamEvent::InProgress(ResponseSnapshotEvent {
            sequence_number: in_progress_sequence_number,
            response: self.builder.in_progress(),
        }));
    }

    fn handle_done(
        &self,
        state: &mut ResponsesStreamingState,
        events: &mut Vec<ResponsesStreamEvent>,
        summary: &GenerationSummary,
    ) {
        state.close_open_item(events);

        let output = state.finalized_output.clone();
        let completed_sequence_number = state.next_sequence_number();
        events.push(ResponsesStreamEvent::Completed(ResponseSnapshotEvent {
            sequence_number: completed_sequence_number,
            response: self.builder.completed(output, &summary.usage),
        }));
    }
}

#[async_trait]
impl TransformsOutgoingMessage for ResponsesStreamingResponseTransformer {
    type Output = ResponsesStreamEvent;

    async fn transform(&self, message: OutgoingMessage) -> Result<Vec<ResponsesStreamEvent>> {
        let mut events: Vec<ResponsesStreamEvent> = Vec::new();
        let mut state = self.state.lock();

        if let Some(error) = responses_error(&message) {
            self.ensure_preamble(&mut state, &mut events);

            let failed_sequence_number = state.next_sequence_number();
            events.push(ResponsesStreamEvent::Failed(ResponseSnapshotEvent {
                sequence_number: failed_sequence_number,
                response: self.builder.failed(&error),
            }));

            return Ok(events);
        }

        match message {
            OutgoingMessage::Response(ResponseEnvelope {
                response: OutgoingResponse::GeneratedToken(token),
                ..
            }) => match token {
                GeneratedTokenResult::ContentToken(text)
                | GeneratedTokenResult::UndeterminableToken(text) => {
                    self.ensure_preamble(&mut state, &mut events);
                    state.handle_content(&mut events, &text);
                }
                GeneratedTokenResult::ReasoningToken(text) => {
                    self.ensure_preamble(&mut state, &mut events);
                    state.handle_reasoning(&mut events, &text);
                }
                GeneratedTokenResult::ToolCallToken(_) => {}
                GeneratedTokenResult::ToolCallParsed(parsed_calls) => {
                    self.ensure_preamble(&mut state, &mut events);
                    state.handle_tool_calls(&mut events, &parsed_calls)?;
                }
                GeneratedTokenResult::Done(summary) => {
                    self.ensure_preamble(&mut state, &mut events);
                    self.handle_done(&mut state, &mut events, &summary);
                }
                other => {
                    return Err(anyhow!(
                        "ResponsesStreamingResponseTransformer received a token it does not know how to handle: {other:?}"
                    ));
                }
            },
            other => {
                return Err(anyhow!(
                    "ResponsesStreamingResponseTransformer received an outgoing message it does not know how to handle: {other:?}"
                ));
            }
        }

        Ok(events)
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

    use crate::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;
    use crate::compatibility::openai_service::responses_response_builder::ResponsesResponseBuilder;
    use crate::compatibility::openai_service::responses_stream_event::ResponsesStreamEvent;

    use super::ResponsesStreamingResponseTransformer;
    use super::ResponsesStreamingState;

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

    fn streaming_transformer() -> ResponsesStreamingResponseTransformer {
        ResponsesStreamingResponseTransformer {
            builder: builder(),
            state: Arc::new(Mutex::new(ResponsesStreamingState::default())),
        }
    }

    fn names(events: &[ResponsesStreamEvent]) -> Vec<&'static str> {
        events
            .iter()
            .map(ResponsesStreamEvent::event_name)
            .collect()
    }

    #[tokio::test]
    async fn streaming_first_content_token_emits_preamble_then_text_delta() {
        let transformer = streaming_transformer();

        let events = transformer
            .transform(token_message(GeneratedTokenResult::ContentToken(
                "hi".to_owned(),
            )))
            .await
            .unwrap();

        assert_eq!(
            names(&events),
            vec![
                "response.created",
                "response.in_progress",
                "response.output_item.added",
                "response.content_part.added",
                "response.output_text.delta",
            ]
        );
        assert_eq!(events[0].to_json()["response"]["status"], "in_progress");
        assert_eq!(events[4].to_json()["delta"], "hi");
    }

    #[tokio::test]
    async fn streaming_preamble_is_emitted_only_once() {
        let transformer = streaming_transformer();

        transformer
            .transform(token_message(GeneratedTokenResult::ContentToken(
                "a".to_owned(),
            )))
            .await
            .unwrap();
        let events = transformer
            .transform(token_message(GeneratedTokenResult::ContentToken(
                "b".to_owned(),
            )))
            .await
            .unwrap();

        assert_eq!(names(&events), vec!["response.output_text.delta"]);
    }

    #[tokio::test]
    async fn streaming_done_finalizes_message_and_emits_completed_with_usage() {
        let transformer = streaming_transformer();

        transformer
            .transform(token_message(GeneratedTokenResult::ContentToken(
                "hello".to_owned(),
            )))
            .await
            .unwrap();
        let events = transformer
            .transform(token_message(GeneratedTokenResult::Done(
                summary_with_counts(7, 4, 1),
            )))
            .await
            .unwrap();

        assert_eq!(
            names(&events),
            vec![
                "response.output_text.done",
                "response.content_part.done",
                "response.output_item.done",
                "response.completed",
            ]
        );

        let completed = events[3].to_json();

        assert_eq!(completed["response"]["status"], "completed");
        assert_eq!(completed["response"]["usage"]["input_tokens"], 7);
        assert_eq!(completed["response"]["usage"]["total_tokens"], 12);
        assert_eq!(
            completed["response"]["output"][0]["content"][0]["text"],
            "hello"
        );
        assert_eq!(
            completed["response"]["output"][0]["content"][0]["logprobs"],
            json!([])
        );
    }

    #[tokio::test]
    async fn streaming_reasoning_then_content_closes_the_reasoning_item_first() {
        let transformer = streaming_transformer();

        transformer
            .transform(token_message(GeneratedTokenResult::ReasoningToken(
                "think".to_owned(),
            )))
            .await
            .unwrap();
        let events = transformer
            .transform(token_message(GeneratedTokenResult::ContentToken(
                "answer".to_owned(),
            )))
            .await
            .unwrap();

        assert_eq!(
            names(&events),
            vec![
                "response.reasoning_text.done",
                "response.output_item.done",
                "response.output_item.added",
                "response.content_part.added",
                "response.output_text.delta",
            ]
        );
        // reasoning item closed at output_index 0, message opened at output_index 1
        assert_eq!(events[1].to_json()["output_index"], 0);
        assert_eq!(events[2].to_json()["output_index"], 1);
        assert_eq!(events[1].to_json()["item"]["type"], "reasoning");
    }

    #[tokio::test]
    async fn streaming_tool_call_emits_function_call_argument_events_without_content_index() {
        let transformer = streaming_transformer();

        let events = transformer
            .transform(token_message(GeneratedTokenResult::ToolCallParsed(vec![
                weather_call(),
            ])))
            .await
            .unwrap();

        assert_eq!(
            names(&events),
            vec![
                "response.created",
                "response.in_progress",
                "response.output_item.added",
                "response.function_call_arguments.delta",
                "response.function_call_arguments.done",
                "response.output_item.done",
            ]
        );

        let delta_event = events[3].to_json();

        assert_eq!(delta_event["delta"], "{\"location\":\"Paris\"}");
        assert!(
            delta_event.get("content_index").is_none(),
            "function_call_arguments events must not carry a content_index"
        );
        assert_eq!(events[4].to_json()["name"], "get_weather");
        assert_eq!(events[5].to_json()["item"]["call_id"], "call_x");
    }

    #[tokio::test]
    async fn streaming_error_emits_preamble_then_failed() {
        let transformer = streaming_transformer();

        let events = transformer
            .transform(token_message(GeneratedTokenResult::ChatTemplateError(
                "boom".to_owned(),
            )))
            .await
            .unwrap();

        assert_eq!(
            names(&events),
            vec![
                "response.created",
                "response.in_progress",
                "response.failed"
            ]
        );

        let failed = events[2].to_json();

        assert_eq!(failed["response"]["status"], "failed");
        assert_eq!(failed["response"]["error"]["code"], "server_error");
        assert_eq!(failed["response"]["error"]["message"], "boom");
    }

    #[tokio::test]
    async fn every_emitted_streaming_event_conforms_to_the_official_schema() {
        let validator = OpenAIValidator::new().unwrap();
        let transformer = streaming_transformer();

        let mut emitted: Vec<ResponsesStreamEvent> = Vec::new();

        for token in [
            GeneratedTokenResult::ReasoningToken("ponder".to_owned()),
            GeneratedTokenResult::ContentToken("hello".to_owned()),
            GeneratedTokenResult::ToolCallParsed(vec![weather_call()]),
            GeneratedTokenResult::Done(summary_with_counts(5, 3, 2)),
        ] {
            emitted.extend(transformer.transform(token_message(token)).await.unwrap());
        }

        assert!(emitted.len() > 10);

        for event in &emitted {
            validator
                .validate_responses_stream_event(&event.to_json())
                .unwrap();
        }
    }

    #[tokio::test]
    async fn the_failed_streaming_event_conforms_to_the_official_schema() {
        let validator = OpenAIValidator::new().unwrap();
        let transformer = streaming_transformer();

        let events = transformer
            .transform(token_message(GeneratedTokenResult::SamplerError(
                "boom".to_owned(),
            )))
            .await
            .unwrap();

        for event in &events {
            validator
                .validate_responses_stream_event(&event.to_json())
                .unwrap();
        }
    }
}
