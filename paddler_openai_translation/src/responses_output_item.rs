use llama_cpp_bindings_types::ParsedToolCall;
use serde::Serialize;

use crate::assistant_role::ASSISTANT_ROLE;
use crate::responses_content_part::ResponsesContentPart;
use crate::responses_item_status::ResponsesItemStatus;
use crate::responses_reasoning_part::ResponsesReasoningPart;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ResponsesOutputItem {
    FunctionCall {
        arguments: String,
        call_id: String,
        id: String,
        name: String,
        status: ResponsesItemStatus,
    },
    Message {
        content: Vec<ResponsesContentPart>,
        id: String,
        role: &'static str,
        status: ResponsesItemStatus,
    },
    Reasoning {
        content: Vec<ResponsesReasoningPart>,
        id: String,
        status: ResponsesItemStatus,
        summary: [u8; 0],
    },
}

impl ResponsesOutputItem {
    #[must_use]
    pub fn completed_function_call(
        id: String,
        parsed_tool_call: &ParsedToolCall,
        arguments: String,
    ) -> Self {
        Self::FunctionCall {
            arguments,
            call_id: parsed_tool_call.id.clone(),
            id,
            name: parsed_tool_call.name.clone(),
            status: ResponsesItemStatus::Completed,
        }
    }

    #[must_use]
    pub fn function_call_in_progress(id: String, parsed_tool_call: &ParsedToolCall) -> Self {
        Self::FunctionCall {
            arguments: String::new(),
            call_id: parsed_tool_call.id.clone(),
            id,
            name: parsed_tool_call.name.clone(),
            status: ResponsesItemStatus::InProgress,
        }
    }

    #[must_use]
    pub fn completed_message(id: String, text: String) -> Self {
        Self::Message {
            content: vec![ResponsesContentPart::output_text(text)],
            id,
            role: ASSISTANT_ROLE,
            status: ResponsesItemStatus::Completed,
        }
    }

    #[must_use]
    pub const fn message_in_progress(id: String) -> Self {
        Self::Message {
            content: Vec::new(),
            id,
            role: ASSISTANT_ROLE,
            status: ResponsesItemStatus::InProgress,
        }
    }

    #[must_use]
    pub fn completed_reasoning(id: String, text: String) -> Self {
        Self::Reasoning {
            content: vec![ResponsesReasoningPart::ReasoningText { text }],
            id,
            status: ResponsesItemStatus::Completed,
            summary: [],
        }
    }

    #[must_use]
    pub const fn reasoning_in_progress(id: String) -> Self {
        Self::Reasoning {
            content: Vec::new(),
            id,
            status: ResponsesItemStatus::InProgress,
            summary: [],
        }
    }
}
