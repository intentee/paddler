use std::mem::take;

use llama_cpp_bindings_types::ParsedToolCall;

use paddler_messaging::generation_summary::GenerationSummary;

use crate::arguments_to_tool_call_string::arguments_to_tool_call_string;
use crate::content_part_event::ContentPartEvent;
use crate::function_call_arguments_delta_event::FunctionCallArgumentsDeltaEvent;
use crate::function_call_arguments_done_event::FunctionCallArgumentsDoneEvent;
use crate::generated_output_part::GeneratedOutputPart;
use crate::generation_event::GenerationEvent;
use crate::generation_failure::GenerationFailure;
use crate::open_item::OpenItem;
use crate::output_item_event::OutputItemEvent;
use crate::response_snapshot_event::ResponseSnapshotEvent;
use crate::responses_content_part::ResponsesContentPart;
use crate::responses_output_item::ResponsesOutputItem;
use crate::responses_output_item_kind::ResponsesOutputItemKind;
use crate::responses_response_header::ResponsesResponseHeader;
use crate::responses_stream_event::ResponsesStreamEvent;
use crate::text_delta_event::TextDeltaEvent;
use crate::text_done_event::TextDoneEvent;

pub struct ResponsesStream {
    finalized_output: Vec<ResponsesOutputItem>,
    header: ResponsesResponseHeader,
    open: OpenItem,
    output_index: usize,
    sequence_number: u64,
    started: bool,
}

impl ResponsesStream {
    #[must_use]
    pub const fn new(header: ResponsesResponseHeader) -> Self {
        Self {
            finalized_output: Vec::new(),
            header,
            open: OpenItem::Nothing,
            output_index: 0,
            sequence_number: 0,
            started: false,
        }
    }

    pub fn advance(&mut self, generation_event: GenerationEvent) -> Vec<ResponsesStreamEvent> {
        let mut events: Vec<ResponsesStreamEvent> = Vec::new();

        match generation_event {
            GenerationEvent::Produced(GeneratedOutputPart::Content(text)) => {
                self.ensure_preamble(&mut events);
                self.handle_content(&mut events, &text);
            }
            GenerationEvent::Produced(GeneratedOutputPart::Reasoning(text)) => {
                self.ensure_preamble(&mut events);
                self.handle_reasoning(&mut events, &text);
            }
            GenerationEvent::Produced(GeneratedOutputPart::ToolCalls(parsed_calls)) => {
                self.ensure_preamble(&mut events);
                self.handle_tool_calls(&mut events, &parsed_calls);
            }
            GenerationEvent::ToolCallTokenProduced => {}
            GenerationEvent::Finished(generation_summary) => {
                self.ensure_preamble(&mut events);
                self.handle_done(&mut events, &generation_summary);
            }
            GenerationEvent::Failed(GenerationFailure { message, .. }) => {
                return self.fail(message);
            }
        }

        events
    }

    pub fn fail(&mut self, error_message: String) -> Vec<ResponsesStreamEvent> {
        let mut events: Vec<ResponsesStreamEvent> = Vec::new();

        self.ensure_preamble(&mut events);

        let failed_sequence_number = self.next_sequence_number();

        events.push(ResponsesStreamEvent::Failed(ResponseSnapshotEvent {
            sequence_number: failed_sequence_number,
            response: self.header.failed(error_message),
        }));

        events
    }

    fn ensure_preamble(&mut self, events: &mut Vec<ResponsesStreamEvent>) {
        if self.started {
            return;
        }

        self.started = true;

        let created_sequence_number = self.next_sequence_number();

        events.push(ResponsesStreamEvent::Created(ResponseSnapshotEvent {
            sequence_number: created_sequence_number,
            response: self.header.in_progress(),
        }));

        let in_progress_sequence_number = self.next_sequence_number();

        events.push(ResponsesStreamEvent::InProgress(ResponseSnapshotEvent {
            sequence_number: in_progress_sequence_number,
            response: self.header.in_progress(),
        }));
    }

    fn handle_done(
        &mut self,
        events: &mut Vec<ResponsesStreamEvent>,
        GenerationSummary { finish, usage }: &GenerationSummary,
    ) {
        self.close_open_item(events);

        let output = take(&mut self.finalized_output);
        let snapshot = ResponseSnapshotEvent {
            sequence_number: self.next_sequence_number(),
            response: self.header.finished(output, usage, *finish),
        };

        events.push(if finish.reached_a_length_limit() {
            ResponsesStreamEvent::Incomplete(snapshot)
        } else {
            ResponsesStreamEvent::Completed(snapshot)
        });
    }

    const fn next_sequence_number(&mut self) -> u64 {
        let sequence_number = self.sequence_number;
        self.sequence_number += 1;

        sequence_number
    }

    fn close_open_item(&mut self, events: &mut Vec<ResponsesStreamEvent>) {
        let output_index = self.output_index;
        let item = match take(&mut self.open) {
            OpenItem::Nothing => return,
            OpenItem::Reasoning { item_id, text } => {
                let text_done_sequence_number = self.next_sequence_number();
                events.push(ResponsesStreamEvent::ReasoningTextDone(TextDoneEvent {
                    sequence_number: text_done_sequence_number,
                    item_id: item_id.clone(),
                    output_index,
                    content_index: 0,
                    text: text.clone(),
                }));

                ResponsesOutputItem::completed_reasoning(item_id, text)
            }
            OpenItem::Message { item_id, text } => {
                let text_done_sequence_number = self.next_sequence_number();
                events.push(ResponsesStreamEvent::OutputTextDone(TextDoneEvent {
                    sequence_number: text_done_sequence_number,
                    item_id: item_id.clone(),
                    output_index,
                    content_index: 0,
                    text: text.clone(),
                }));

                let part_done_sequence_number = self.next_sequence_number();
                events.push(ResponsesStreamEvent::ContentPartDone(ContentPartEvent {
                    sequence_number: part_done_sequence_number,
                    item_id: item_id.clone(),
                    output_index,
                    content_index: 0,
                    part: ResponsesContentPart::output_text(text.clone()),
                }));

                ResponsesOutputItem::completed_message(item_id, text)
            }
        };

        self.finish_item(events, item);
    }

    fn handle_reasoning(&mut self, events: &mut Vec<ResponsesStreamEvent>, text: &str) {
        let item_id = if let OpenItem::Reasoning {
            item_id,
            text: reasoning_text,
        } = &mut self.open
        {
            reasoning_text.push_str(text);

            item_id.clone()
        } else {
            self.close_open_item(events);

            let item_id = ResponsesOutputItemKind::Reasoning.item_id(self.output_index);

            self.add_item(
                events,
                ResponsesOutputItem::reasoning_in_progress(item_id.clone()),
            );
            self.open = OpenItem::Reasoning {
                item_id: item_id.clone(),
                text: text.to_owned(),
            };

            item_id
        };

        let delta_sequence_number = self.next_sequence_number();
        events.push(ResponsesStreamEvent::ReasoningTextDelta(TextDeltaEvent {
            sequence_number: delta_sequence_number,
            item_id,
            output_index: self.output_index,
            content_index: 0,
            delta: text.to_owned(),
        }));
    }

    fn handle_content(&mut self, events: &mut Vec<ResponsesStreamEvent>, text: &str) {
        let item_id = if let OpenItem::Message {
            item_id,
            text: message_text,
        } = &mut self.open
        {
            message_text.push_str(text);

            item_id.clone()
        } else {
            self.close_open_item(events);

            let item_id = ResponsesOutputItemKind::Message.item_id(self.output_index);

            self.add_item(
                events,
                ResponsesOutputItem::message_in_progress(item_id.clone()),
            );

            let part_added_sequence_number = self.next_sequence_number();
            events.push(ResponsesStreamEvent::ContentPartAdded(ContentPartEvent {
                sequence_number: part_added_sequence_number,
                item_id: item_id.clone(),
                output_index: self.output_index,
                content_index: 0,
                part: ResponsesContentPart::output_text(String::new()),
            }));
            self.open = OpenItem::Message {
                item_id: item_id.clone(),
                text: text.to_owned(),
            };

            item_id
        };

        let delta_sequence_number = self.next_sequence_number();
        events.push(ResponsesStreamEvent::OutputTextDelta(TextDeltaEvent {
            sequence_number: delta_sequence_number,
            item_id,
            output_index: self.output_index,
            content_index: 0,
            delta: text.to_owned(),
        }));
    }

    fn handle_tool_calls(
        &mut self,
        events: &mut Vec<ResponsesStreamEvent>,
        parsed_calls: &[ParsedToolCall],
    ) {
        self.close_open_item(events);

        for call in parsed_calls {
            let output_index = self.output_index;
            let item_id = ResponsesOutputItemKind::FunctionCall.item_id(output_index);
            let arguments = arguments_to_tool_call_string(&call.arguments);

            self.add_item(
                events,
                ResponsesOutputItem::function_call_in_progress(item_id.clone(), call),
            );

            let delta_sequence_number = self.next_sequence_number();
            events.push(ResponsesStreamEvent::FunctionCallArgumentsDelta(
                FunctionCallArgumentsDeltaEvent {
                    sequence_number: delta_sequence_number,
                    item_id: item_id.clone(),
                    output_index,
                    delta: arguments.clone(),
                },
            ));

            let done_sequence_number = self.next_sequence_number();
            events.push(ResponsesStreamEvent::FunctionCallArgumentsDone(
                FunctionCallArgumentsDoneEvent {
                    sequence_number: done_sequence_number,
                    item_id: item_id.clone(),
                    output_index,
                    name: call.name.clone(),
                    arguments: arguments.clone(),
                },
            ));

            self.finish_item(
                events,
                ResponsesOutputItem::completed_function_call(item_id, call, arguments),
            );
        }
    }

    fn add_item(&mut self, events: &mut Vec<ResponsesStreamEvent>, item: ResponsesOutputItem) {
        let added_sequence_number = self.next_sequence_number();

        events.push(ResponsesStreamEvent::OutputItemAdded(OutputItemEvent {
            sequence_number: added_sequence_number,
            output_index: self.output_index,
            item,
        }));
    }

    fn finish_item(&mut self, events: &mut Vec<ResponsesStreamEvent>, item: ResponsesOutputItem) {
        let item_done_sequence_number = self.next_sequence_number();

        events.push(ResponsesStreamEvent::OutputItemDone(OutputItemEvent {
            sequence_number: item_done_sequence_number,
            output_index: self.output_index,
            item: item.clone(),
        }));

        self.finalized_output.push(item);
        self.output_index += 1;
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings_types::ParsedToolCall;
    use llama_cpp_bindings_types::TokenUsage;
    use llama_cpp_bindings_types::ToolCallArguments;
    use serde_json::Value;
    use serde_json::json;
    use serde_json::to_value;

    use paddler_messaging::generation_finish::GenerationFinish;
    use paddler_messaging::generation_summary::GenerationSummary;
    use paddler_openai_response_format_validator::openai_validator::OpenAIValidator;

    use super::ResponsesStream;
    use crate::generated_output_part::GeneratedOutputPart;
    use crate::generation_event::GenerationEvent;
    use crate::generation_failure::GenerationFailure;
    use crate::generation_failure_cause::GenerationFailureCause;
    use crate::responses_response_header::ResponsesResponseHeader;
    use crate::responses_stream_event::ResponsesStreamEvent;

    fn responses_stream() -> ResponsesStream {
        ResponsesStream::new(ResponsesResponseHeader {
            id: "resp_test".to_owned(),
            created_at: 0,
            model: "test-model".to_owned(),
            instructions: None,
            temperature: 0.25,
            top_p: 0.5,
        })
    }

    fn content(text: &str) -> GenerationEvent {
        GenerationEvent::Produced(GeneratedOutputPart::Content(text.to_owned()))
    }

    fn finished(finish: GenerationFinish) -> GenerationEvent {
        GenerationEvent::Finished(GenerationSummary {
            finish,
            usage: TokenUsage {
                prompt_tokens: 7,
                content_tokens: 4,
                reasoning_tokens: 1,
                ..TokenUsage::default()
            },
        })
    }

    fn weather_call() -> ParsedToolCall {
        ParsedToolCall::new(
            "call_x".to_owned(),
            "get_weather".to_owned(),
            ToolCallArguments::ValidJson(json!({ "location": "Paris" })),
        )
    }

    fn serialized(event: &ResponsesStreamEvent) -> Value {
        to_value(event).unwrap()
    }

    fn names(events: &[ResponsesStreamEvent]) -> Vec<&'static str> {
        events
            .iter()
            .map(ResponsesStreamEvent::event_name)
            .collect()
    }

    #[test]
    fn the_first_content_emits_the_preamble_then_a_text_delta() {
        let events = responses_stream().advance(content("hi"));

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
        assert_eq!(serialized(&events[0])["response"]["status"], "in_progress");
        assert_eq!(serialized(&events[4])["delta"], "hi");
    }

    #[test]
    fn the_preamble_is_emitted_only_once() {
        let mut responses_stream = responses_stream();

        responses_stream.advance(content("a"));

        assert_eq!(
            names(&responses_stream.advance(content("b"))),
            vec!["response.output_text.delta"]
        );
    }

    #[test]
    fn tool_call_tokens_do_not_start_the_response() {
        assert!(
            responses_stream()
                .advance(GenerationEvent::ToolCallTokenProduced)
                .is_empty()
        );
    }

    #[test]
    fn finishing_closes_the_message_and_completes_with_usage() {
        let mut responses_stream = responses_stream();

        responses_stream.advance(content("hello"));

        let events = responses_stream.advance(finished(GenerationFinish::EndOfGeneration));

        assert_eq!(
            names(&events),
            vec![
                "response.output_text.done",
                "response.content_part.done",
                "response.output_item.done",
                "response.completed",
            ]
        );

        let completed = serialized(&events[3]);

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

    #[test]
    fn finishing_at_a_length_limit_reports_an_incomplete_response() {
        let events = responses_stream().advance(finished(GenerationFinish::MaxTokens));

        assert_eq!(
            names(&events),
            vec![
                "response.created",
                "response.in_progress",
                "response.incomplete"
            ]
        );
        assert_eq!(
            serialized(&events[2])["response"]["incomplete_details"]["reason"],
            "max_output_tokens"
        );
    }

    #[test]
    fn reasoning_then_content_closes_the_reasoning_item_first() {
        let mut responses_stream = responses_stream();

        responses_stream.advance(GenerationEvent::Produced(GeneratedOutputPart::Reasoning(
            "think".to_owned(),
        )));

        let events = responses_stream.advance(content("answer"));

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
        assert_eq!(serialized(&events[1])["output_index"], 0);
        assert_eq!(serialized(&events[2])["output_index"], 1);
        assert_eq!(serialized(&events[1])["item"]["type"], "reasoning");
    }

    #[test]
    fn tool_calls_emit_function_call_argument_events_without_a_content_index() {
        let events = responses_stream().advance(GenerationEvent::Produced(
            GeneratedOutputPart::ToolCalls(vec![weather_call()]),
        ));

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

        let delta_event = serialized(&events[3]);

        assert_eq!(delta_event["delta"], "{\"location\":\"Paris\"}");
        assert!(delta_event.get("content_index").is_none());
        assert_eq!(serialized(&events[4])["name"], "get_weather");
        assert_eq!(serialized(&events[5])["item"]["call_id"], "call_x");
    }

    #[test]
    fn a_generation_failure_emits_the_preamble_then_response_failed() {
        let events = responses_stream().advance(GenerationEvent::Failed(GenerationFailure {
            cause: GenerationFailureCause::AgentFailed,
            message: "boom".to_owned(),
        }));

        assert_eq!(
            names(&events),
            vec![
                "response.created",
                "response.in_progress",
                "response.failed"
            ]
        );

        let failed = serialized(&events[2]);

        assert_eq!(failed["response"]["status"], "failed");
        assert_eq!(failed["response"]["error"]["code"], "server_error");
        assert_eq!(failed["response"]["error"]["message"], "boom");
    }

    #[test]
    fn every_emitted_event_conforms_to_the_schema() {
        let validator = OpenAIValidator::new().unwrap();
        let mut completed_stream = responses_stream();
        let mut emitted: Vec<ResponsesStreamEvent> = Vec::new();

        for generation_event in [
            GenerationEvent::Produced(GeneratedOutputPart::Reasoning("ponder".to_owned())),
            content("hello"),
            GenerationEvent::Produced(GeneratedOutputPart::ToolCalls(vec![weather_call()])),
            finished(GenerationFinish::EndOfGeneration),
        ] {
            emitted.extend(completed_stream.advance(generation_event));
        }

        emitted.extend(responses_stream().fail("boom".to_owned()));

        for event in &emitted {
            validator
                .validate_responses_stream_event(&serialized(event))
                .unwrap();
        }
    }
}
