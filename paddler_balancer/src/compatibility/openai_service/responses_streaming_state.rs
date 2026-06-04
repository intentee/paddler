use anyhow::Result;
use llama_cpp_bindings_types::ParsedToolCall;
use serde_json::Value;
use serde_json::json;

use crate::compatibility::openai_service::arguments_to_tool_call_string::arguments_to_tool_call_string;
use crate::compatibility::openai_service::content_part_event::ContentPartEvent;
use crate::compatibility::openai_service::function_call_arguments_delta_event::FunctionCallArgumentsDeltaEvent;
use crate::compatibility::openai_service::function_call_arguments_done_event::FunctionCallArgumentsDoneEvent;
use crate::compatibility::openai_service::function_call_item::function_call_item;
use crate::compatibility::openai_service::message_item_done::message_item_done;
use crate::compatibility::openai_service::open_item::OpenItem;
use crate::compatibility::openai_service::output_item_event::OutputItemEvent;
use crate::compatibility::openai_service::output_text_part::output_text_part;
use crate::compatibility::openai_service::reasoning_item_done::reasoning_item_done;
use crate::compatibility::openai_service::responses_stream_event::ResponsesStreamEvent;
use crate::compatibility::openai_service::text_delta_event::TextDeltaEvent;
use crate::compatibility::openai_service::text_done_event::TextDoneEvent;

fn message_item_open(item_id: &str) -> Value {
    json!({
        "id": item_id,
        "type": "message",
        "role": "assistant",
        "status": "in_progress",
        "content": []
    })
}

fn reasoning_item_open(item_id: &str) -> Value {
    json!({
        "type": "reasoning",
        "id": item_id,
        "summary": [],
        "status": "in_progress"
    })
}

#[derive(Default)]
pub struct ResponsesStreamingState {
    pub started: bool,
    sequence_number: u64,
    output_index: usize,
    open: OpenItem,
    reasoning_id: String,
    reasoning_text: String,
    message_id: String,
    message_text: String,
    pub finalized_output: Vec<Value>,
}

impl ResponsesStreamingState {
    pub const fn next_sequence_number(&mut self) -> u64 {
        let sequence_number = self.sequence_number;
        self.sequence_number += 1;

        sequence_number
    }

    pub fn close_open_item(&mut self, events: &mut Vec<ResponsesStreamEvent>) {
        match self.open {
            OpenItem::None => {}
            OpenItem::Reasoning => {
                let item_id = self.reasoning_id.clone();
                let text = self.reasoning_text.clone();
                let output_index = self.output_index;

                let text_done_sequence_number = self.next_sequence_number();
                events.push(ResponsesStreamEvent::ReasoningTextDone(TextDoneEvent {
                    sequence_number: text_done_sequence_number,
                    item_id: item_id.clone(),
                    output_index,
                    content_index: 0,
                    text: text.clone(),
                }));

                let item = reasoning_item_done(&item_id, &text);
                let item_done_sequence_number = self.next_sequence_number();
                events.push(ResponsesStreamEvent::OutputItemDone(OutputItemEvent {
                    sequence_number: item_done_sequence_number,
                    output_index,
                    item: item.clone(),
                }));

                self.finalized_output.push(item);
                self.output_index += 1;
                self.reasoning_text.clear();
                self.open = OpenItem::None;
            }
            OpenItem::Message => {
                let item_id = self.message_id.clone();
                let text = self.message_text.clone();
                let output_index = self.output_index;

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
                    part: output_text_part(&text),
                }));

                let item = message_item_done(&item_id, &text);
                let item_done_sequence_number = self.next_sequence_number();
                events.push(ResponsesStreamEvent::OutputItemDone(OutputItemEvent {
                    sequence_number: item_done_sequence_number,
                    output_index,
                    item: item.clone(),
                }));

                self.finalized_output.push(item);
                self.output_index += 1;
                self.message_text.clear();
                self.open = OpenItem::None;
            }
        }
    }

    pub fn handle_reasoning(&mut self, events: &mut Vec<ResponsesStreamEvent>, text: &str) {
        if self.open != OpenItem::Reasoning {
            self.close_open_item(events);

            let output_index = self.output_index;
            let item_id = format!("rs_{output_index}");
            self.reasoning_id.clone_from(&item_id);

            let added_sequence_number = self.next_sequence_number();
            events.push(ResponsesStreamEvent::OutputItemAdded(OutputItemEvent {
                sequence_number: added_sequence_number,
                output_index,
                item: reasoning_item_open(&item_id),
            }));

            self.open = OpenItem::Reasoning;
        }

        self.reasoning_text.push_str(text);

        let item_id = self.reasoning_id.clone();
        let output_index = self.output_index;
        let delta_sequence_number = self.next_sequence_number();
        events.push(ResponsesStreamEvent::ReasoningTextDelta(TextDeltaEvent {
            sequence_number: delta_sequence_number,
            item_id,
            output_index,
            content_index: 0,
            delta: text.to_owned(),
        }));
    }

    pub fn handle_content(&mut self, events: &mut Vec<ResponsesStreamEvent>, text: &str) {
        if self.open != OpenItem::Message {
            self.close_open_item(events);

            let output_index = self.output_index;
            let item_id = format!("msg_{output_index}");
            self.message_id.clone_from(&item_id);

            let added_sequence_number = self.next_sequence_number();
            events.push(ResponsesStreamEvent::OutputItemAdded(OutputItemEvent {
                sequence_number: added_sequence_number,
                output_index,
                item: message_item_open(&item_id),
            }));

            let part_added_sequence_number = self.next_sequence_number();
            events.push(ResponsesStreamEvent::ContentPartAdded(ContentPartEvent {
                sequence_number: part_added_sequence_number,
                item_id,
                output_index,
                content_index: 0,
                part: output_text_part(""),
            }));

            self.open = OpenItem::Message;
        }

        self.message_text.push_str(text);

        let item_id = self.message_id.clone();
        let output_index = self.output_index;
        let delta_sequence_number = self.next_sequence_number();
        events.push(ResponsesStreamEvent::OutputTextDelta(TextDeltaEvent {
            sequence_number: delta_sequence_number,
            item_id,
            output_index,
            content_index: 0,
            delta: text.to_owned(),
        }));
    }

    pub fn handle_tool_calls(
        &mut self,
        events: &mut Vec<ResponsesStreamEvent>,
        parsed_calls: &[ParsedToolCall],
    ) -> Result<()> {
        self.close_open_item(events);

        for call in parsed_calls {
            let output_index = self.output_index;
            let item_id = format!("fc_{output_index}");
            let arguments = arguments_to_tool_call_string(&call.arguments)?;

            let added_sequence_number = self.next_sequence_number();
            events.push(ResponsesStreamEvent::OutputItemAdded(OutputItemEvent {
                sequence_number: added_sequence_number,
                output_index,
                item: function_call_item(&item_id, &call.id, &call.name, "", "in_progress"),
            }));

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

            let item = function_call_item(&item_id, &call.id, &call.name, &arguments, "completed");
            let item_done_sequence_number = self.next_sequence_number();
            events.push(ResponsesStreamEvent::OutputItemDone(OutputItemEvent {
                sequence_number: item_done_sequence_number,
                output_index,
                item: item.clone(),
            }));

            self.finalized_output.push(item);
            self.output_index += 1;
        }

        Ok(())
    }
}
