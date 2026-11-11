use std::mem::take;
use std::ops::RangeFrom;

use paddler_messaging::generated_tool_call::GeneratedToolCall;
use paddler_messaging::generation_summary::GenerationSummary;

use crate::arguments_to_tool_call_string::arguments_to_tool_call_string;
use crate::content_part_event::ContentPartEvent;
use crate::function_call_arguments_delta_event::FunctionCallArgumentsDeltaEvent;
use crate::function_call_arguments_done_event::FunctionCallArgumentsDoneEvent;
use crate::generated_output_part::GeneratedOutputPart;
use crate::generation_failure::GenerationFailure;
use crate::generation_outcome::GenerationOutcome;
use crate::generation_progress::GenerationProgress;
use crate::open_item::OpenItem;
use crate::output_item_event::OutputItemEvent;
use crate::response_snapshot_event::ResponseSnapshotEvent;
use crate::responses_content_part::ResponsesContentPart;
use crate::responses_output_item::ResponsesOutputItem;
use crate::responses_output_item_kind::ResponsesOutputItemKind;
use crate::responses_response_header::ResponsesResponseHeader;
use crate::responses_stream_event::ResponsesStreamEvent;
use crate::sequenced_responses_stream_event::SequencedResponsesStreamEvent;
use crate::text_delta_event::TextDeltaEvent;
use crate::text_done_event::TextDoneEvent;

pub struct ResponsesStream {
    finalized_output: Vec<ResponsesOutputItem>,
    header: ResponsesResponseHeader,
    open: OpenItem,
    sequence_numbers: RangeFrom<u64>,
    started: bool,
}

impl ResponsesStream {
    #[must_use]
    pub const fn new(header: ResponsesResponseHeader) -> Self {
        Self {
            finalized_output: Vec::new(),
            header,
            open: OpenItem::Nothing,
            sequence_numbers: 0..,
            started: false,
        }
    }

    pub fn advance(
        &mut self,
        generation_progress: GenerationProgress,
    ) -> Vec<SequencedResponsesStreamEvent> {
        let mut events: Vec<ResponsesStreamEvent> = Vec::new();

        match generation_progress {
            GenerationProgress::Produced(GeneratedOutputPart::Content(text)) => {
                self.ensure_preamble(&mut events);
                self.handle_content(&mut events, &text);
            }
            GenerationProgress::Produced(GeneratedOutputPart::Reasoning(text)) => {
                self.ensure_preamble(&mut events);
                self.handle_reasoning(&mut events, &text);
            }
            GenerationProgress::Produced(GeneratedOutputPart::ToolCalls(parsed_calls)) => {
                self.ensure_preamble(&mut events);
                self.handle_tool_calls(&mut events, &parsed_calls);
            }
            GenerationProgress::ToolCallTokenProduced => {}
            GenerationProgress::Failed(GenerationFailure { message, .. }) => {
                return self.fail(message);
            }
        }

        self.assign_sequence_numbers(events)
    }

    pub fn conclude(
        &mut self,
        generation_outcome: GenerationOutcome,
    ) -> Vec<SequencedResponsesStreamEvent> {
        match generation_outcome {
            GenerationOutcome::Failed(GenerationFailure { message, .. }) => self.fail(message),
            GenerationOutcome::Finished(generation_summary) => {
                let mut events: Vec<ResponsesStreamEvent> = Vec::new();

                self.ensure_preamble(&mut events);
                self.handle_done(&mut events, &generation_summary);

                self.assign_sequence_numbers(events)
            }
        }
    }

    pub fn fail(&mut self, error_message: String) -> Vec<SequencedResponsesStreamEvent> {
        let mut events: Vec<ResponsesStreamEvent> = Vec::new();

        self.ensure_preamble(&mut events);

        events.push(ResponsesStreamEvent::Failed(ResponseSnapshotEvent {
            response: self.header.failed(error_message),
        }));

        self.assign_sequence_numbers(events)
    }

    fn add_item(&self, events: &mut Vec<ResponsesStreamEvent>, item: ResponsesOutputItem) {
        events.push(ResponsesStreamEvent::OutputItemAdded(OutputItemEvent {
            output_index: self.output_index(),
            item,
        }));
    }

    fn assign_sequence_numbers(
        &mut self,
        events: Vec<ResponsesStreamEvent>,
    ) -> Vec<SequencedResponsesStreamEvent> {
        events
            .into_iter()
            .zip(self.sequence_numbers.by_ref())
            .map(|(event, sequence_number)| SequencedResponsesStreamEvent {
                event,
                sequence_number,
            })
            .collect()
    }

    fn close_open_item(&mut self, events: &mut Vec<ResponsesStreamEvent>) {
        let output_index = self.output_index();
        let item = match take(&mut self.open) {
            OpenItem::Nothing => return,
            OpenItem::Reasoning { item_id, text } => {
                events.push(ResponsesStreamEvent::ReasoningTextDone(TextDoneEvent {
                    item_id: item_id.clone(),
                    output_index,
                    content_index: 0,
                    text: text.clone(),
                }));

                ResponsesOutputItem::completed_reasoning(item_id, text)
            }
            OpenItem::Message { item_id, text } => {
                events.push(ResponsesStreamEvent::OutputTextDone(TextDoneEvent {
                    item_id: item_id.clone(),
                    output_index,
                    content_index: 0,
                    text: text.clone(),
                }));

                events.push(ResponsesStreamEvent::ContentPartDone(ContentPartEvent {
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

    fn ensure_preamble(&mut self, events: &mut Vec<ResponsesStreamEvent>) {
        if self.started {
            return;
        }

        self.started = true;

        let in_progress_response = self.header.in_progress();

        events.push(ResponsesStreamEvent::Created(ResponseSnapshotEvent {
            response: in_progress_response.clone(),
        }));
        events.push(ResponsesStreamEvent::InProgress(ResponseSnapshotEvent {
            response: in_progress_response,
        }));
    }

    fn finish_item(&mut self, events: &mut Vec<ResponsesStreamEvent>, item: ResponsesOutputItem) {
        events.push(ResponsesStreamEvent::OutputItemDone(OutputItemEvent {
            output_index: self.output_index(),
            item: item.clone(),
        }));

        self.finalized_output.push(item);
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

            let item_id = ResponsesOutputItemKind::Message.item_id();

            self.add_item(
                events,
                ResponsesOutputItem::message_in_progress(item_id.clone()),
            );

            events.push(ResponsesStreamEvent::ContentPartAdded(ContentPartEvent {
                item_id: item_id.clone(),
                output_index: self.output_index(),
                content_index: 0,
                part: ResponsesContentPart::output_text(String::new()),
            }));
            self.open = OpenItem::Message {
                item_id: item_id.clone(),
                text: text.to_owned(),
            };

            item_id
        };

        events.push(ResponsesStreamEvent::OutputTextDelta(TextDeltaEvent {
            item_id,
            output_index: self.output_index(),
            content_index: 0,
            delta: text.to_owned(),
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
            response: self.header.finished(output, usage, *finish),
        };

        events.push(if finish.reached_a_length_limit() {
            ResponsesStreamEvent::Incomplete(snapshot)
        } else {
            ResponsesStreamEvent::Completed(snapshot)
        });
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

            let item_id = ResponsesOutputItemKind::Reasoning.item_id();

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

        events.push(ResponsesStreamEvent::ReasoningTextDelta(TextDeltaEvent {
            item_id,
            output_index: self.output_index(),
            content_index: 0,
            delta: text.to_owned(),
        }));
    }

    fn handle_tool_calls(
        &mut self,
        events: &mut Vec<ResponsesStreamEvent>,
        parsed_calls: &[GeneratedToolCall],
    ) {
        self.close_open_item(events);

        for call in parsed_calls {
            let output_index = self.output_index();
            let item_id = ResponsesOutputItemKind::FunctionCall.item_id();
            let arguments = arguments_to_tool_call_string(&call.arguments);

            self.add_item(
                events,
                ResponsesOutputItem::function_call_in_progress(item_id.clone(), call),
            );

            events.push(ResponsesStreamEvent::FunctionCallArgumentsDelta(
                FunctionCallArgumentsDeltaEvent {
                    item_id: item_id.clone(),
                    output_index,
                    delta: arguments.clone(),
                },
            ));

            events.push(ResponsesStreamEvent::FunctionCallArgumentsDone(
                FunctionCallArgumentsDoneEvent {
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

    const fn output_index(&self) -> usize {
        self.finalized_output.len()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;
    use serde_json::json;
    use serde_json::to_value;

    use paddler_generation_result_fixture::finished_generation_summary::finished_generation_summary;
    use paddler_generation_result_fixture::weather_call::weather_call;
    use paddler_messaging::generation_finish::GenerationFinish;
    use paddler_openai_response_format_validator::openai_validator::OpenAIValidator;

    use super::ResponsesStream;
    use crate::generated_output_part::GeneratedOutputPart;
    use crate::generation_failure::GenerationFailure;
    use crate::generation_failure_cause::GenerationFailureCause;
    use crate::generation_outcome::GenerationOutcome;
    use crate::generation_progress::GenerationProgress;
    use crate::responses_response_header::ResponsesResponseHeader;
    use crate::sequenced_responses_stream_event::SequencedResponsesStreamEvent;

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

    fn content(text: &str) -> GenerationProgress {
        GenerationProgress::Produced(GeneratedOutputPart::Content(text.to_owned()))
    }

    fn serialized(sequenced_event: &SequencedResponsesStreamEvent) -> Value {
        to_value(sequenced_event).unwrap()
    }

    fn serialized_at(events: &[SequencedResponsesStreamEvent], index: usize) -> Value {
        serialized(
            events
                .get(index)
                .expect("the stream emitted an event at this index"),
        )
    }

    fn names(events: &[SequencedResponsesStreamEvent]) -> Vec<&'static str> {
        events
            .iter()
            .map(|SequencedResponsesStreamEvent { event, .. }| event.event_name())
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
        assert_eq!(
            serialized_at(&events, 0).pointer("/response/status"),
            Some(&json!("in_progress"))
        );
        assert_eq!(
            serialized_at(&events, 4).pointer("/delta"),
            Some(&json!("hi"))
        );
    }

    #[test]
    fn every_event_of_a_streamed_item_carries_the_same_id() {
        let mut responses_stream = responses_stream();
        let mut events = responses_stream.advance(content("hel"));

        events.extend(responses_stream.advance(content("lo")));
        events.extend(responses_stream.conclude(GenerationOutcome::Finished(
            finished_generation_summary(GenerationFinish::EndOfGeneration),
        )));

        let item_ids = events
            .iter()
            .map(serialized)
            .filter_map(|event| match event.get("type").and_then(Value::as_str) {
                Some("response.output_item.added" | "response.output_item.done") => {
                    event.pointer("/item/id").cloned()
                }
                Some("response.completed") => event.pointer("/response/output/0/id").cloned(),
                Some("response.created" | "response.in_progress") => None,
                Some(_) | None => event.get("item_id").cloned(),
            })
            .collect::<Vec<_>>();
        let first_item_id = item_ids.first().expect("the stream names an item");

        assert_eq!(item_ids.len(), 8);
        assert!(item_ids.iter().all(|item_id| item_id == first_item_id));
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
                .advance(GenerationProgress::ToolCallTokenProduced)
                .is_empty()
        );
    }

    #[test]
    fn sequence_numbers_run_from_zero_without_gaps_across_calls() {
        let mut responses_stream = responses_stream();
        let mut events = responses_stream.advance(content("a"));

        events.extend(responses_stream.advance(GenerationProgress::ToolCallTokenProduced));
        events.extend(responses_stream.advance(content("b")));

        assert_eq!(
            events
                .iter()
                .map(|sequenced_event| sequenced_event.sequence_number)
                .collect::<Vec<u64>>(),
            vec![0, 1, 2, 3, 4, 5]
        );
    }

    #[test]
    fn finishing_closes_the_message_and_completes_with_usage() {
        let mut responses_stream = responses_stream();

        responses_stream.advance(content("hello"));

        let events = responses_stream.conclude(GenerationOutcome::Finished(
            finished_generation_summary(GenerationFinish::EndOfGeneration),
        ));

        assert_eq!(
            names(&events),
            vec![
                "response.output_text.done",
                "response.content_part.done",
                "response.output_item.done",
                "response.completed",
            ]
        );

        let completed = serialized_at(&events, 3);

        assert_eq!(
            completed.pointer("/response/status"),
            Some(&json!("completed"))
        );
        assert_eq!(
            completed.pointer("/response/usage/input_tokens"),
            Some(&json!(7))
        );
        assert_eq!(
            completed.pointer("/response/usage/total_tokens"),
            Some(&json!(12))
        );
        assert_eq!(
            completed.pointer("/response/output/0/content/0/text"),
            Some(&json!("hello"))
        );
        assert_eq!(
            completed.pointer("/response/output/0/content/0/logprobs"),
            Some(&json!([]))
        );
    }

    #[test]
    fn finishing_at_a_length_limit_reports_an_incomplete_response() {
        let events = responses_stream().conclude(GenerationOutcome::Finished(
            finished_generation_summary(GenerationFinish::MaxTokens),
        ));

        assert_eq!(
            names(&events),
            vec![
                "response.created",
                "response.in_progress",
                "response.incomplete"
            ]
        );
        assert_eq!(
            serialized_at(&events, 2).pointer("/response/incomplete_details/reason"),
            Some(&json!("max_output_tokens"))
        );
    }

    #[test]
    fn reasoning_then_content_closes_the_reasoning_item_first() {
        let mut responses_stream = responses_stream();

        responses_stream.advance(GenerationProgress::Produced(
            GeneratedOutputPart::Reasoning("think".to_owned()),
        ));

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
        assert_eq!(
            serialized_at(&events, 1).pointer("/output_index"),
            Some(&json!(0))
        );
        assert_eq!(
            serialized_at(&events, 2).pointer("/output_index"),
            Some(&json!(1))
        );
        assert_eq!(
            serialized_at(&events, 1).pointer("/item/type"),
            Some(&json!("reasoning"))
        );
    }

    #[test]
    fn tool_calls_emit_function_call_argument_events_without_a_content_index() {
        let events = responses_stream().advance(GenerationProgress::Produced(
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

        let delta_event = serialized_at(&events, 3);

        assert_eq!(
            delta_event.pointer("/delta"),
            Some(&json!("{\"location\":\"Paris\"}"))
        );
        assert!(delta_event.get("content_index").is_none());
        assert_eq!(
            serialized_at(&events, 4).pointer("/name"),
            Some(&json!("get_weather"))
        );
        assert_eq!(
            serialized_at(&events, 5).pointer("/item/call_id"),
            Some(&json!("call_x"))
        );
    }

    #[test]
    fn a_generation_failure_emits_the_preamble_then_response_failed() {
        let generation_failure = || GenerationFailure {
            cause: GenerationFailureCause::AgentFailed,
            message: "boom".to_owned(),
        };

        for events in [
            responses_stream().advance(GenerationProgress::Failed(generation_failure())),
            responses_stream().conclude(GenerationOutcome::Failed(generation_failure())),
        ] {
            assert_eq!(
                names(&events),
                vec![
                    "response.created",
                    "response.in_progress",
                    "response.failed"
                ]
            );

            let failed = serialized_at(&events, 2);

            assert_eq!(failed.pointer("/response/status"), Some(&json!("failed")));
            assert_eq!(
                failed.pointer("/response/error/code"),
                Some(&json!("server_error"))
            );
            assert_eq!(
                failed.pointer("/response/error/message"),
                Some(&json!("boom"))
            );
        }
    }

    #[test]
    fn every_emitted_event_conforms_to_the_schema() {
        let validator = OpenAIValidator::new().unwrap();
        let mut completed_stream = responses_stream();
        let mut emitted: Vec<SequencedResponsesStreamEvent> = Vec::new();

        for generation_progress in [
            GenerationProgress::Produced(GeneratedOutputPart::Reasoning("ponder".to_owned())),
            content("hello"),
            GenerationProgress::Produced(GeneratedOutputPart::ToolCalls(vec![weather_call()])),
        ] {
            emitted.extend(completed_stream.advance(generation_progress));
        }

        emitted.extend(completed_stream.conclude(GenerationOutcome::Finished(
            finished_generation_summary(GenerationFinish::EndOfGeneration),
        )));

        emitted.extend(responses_stream().fail("boom".to_owned()));

        for event in &emitted {
            validator
                .validate_responses_stream_event(&serialized(event))
                .unwrap();
        }
    }
}
