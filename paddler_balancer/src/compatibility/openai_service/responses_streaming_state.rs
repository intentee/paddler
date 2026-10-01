use std::mem::take;

use llama_cpp_bindings_types::ParsedToolCall;

use crate::compatibility::openai_service::arguments_to_tool_call_string::arguments_to_tool_call_string;
use crate::compatibility::openai_service::content_part_event::ContentPartEvent;
use crate::compatibility::openai_service::function_call_arguments_delta_event::FunctionCallArgumentsDeltaEvent;
use crate::compatibility::openai_service::function_call_arguments_done_event::FunctionCallArgumentsDoneEvent;
use crate::compatibility::openai_service::open_item::OpenItem;
use crate::compatibility::openai_service::output_item_event::OutputItemEvent;
use crate::compatibility::openai_service::responses_content_part::ResponsesContentPart;
use crate::compatibility::openai_service::responses_output_item::ResponsesOutputItem;
use crate::compatibility::openai_service::responses_output_item_kind::ResponsesOutputItemKind;
use crate::compatibility::openai_service::responses_stream_event::ResponsesStreamEvent;
use crate::compatibility::openai_service::text_delta_event::TextDeltaEvent;
use crate::compatibility::openai_service::text_done_event::TextDoneEvent;

#[derive(Default)]
pub struct ResponsesStreamingState {
    pub finalized_output: Vec<ResponsesOutputItem>,
    open: OpenItem,
    output_index: usize,
    sequence_number: u64,
    pub started: bool,
}

impl ResponsesStreamingState {
    pub const fn next_sequence_number(&mut self) -> u64 {
        let sequence_number = self.sequence_number;
        self.sequence_number += 1;

        sequence_number
    }

    pub fn close_open_item(&mut self, events: &mut Vec<ResponsesStreamEvent>) {
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

    pub fn handle_reasoning(&mut self, events: &mut Vec<ResponsesStreamEvent>, text: &str) {
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

    pub fn handle_content(&mut self, events: &mut Vec<ResponsesStreamEvent>, text: &str) {
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

    pub fn handle_tool_calls(
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
                    arguments,
                },
            ));

            self.finish_item(
                events,
                ResponsesOutputItem::completed_function_call(item_id, call),
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
