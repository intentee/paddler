use serde::Deserialize;

use crate::responses_function_call_item::ResponsesFunctionCallItem;
use crate::responses_function_call_output_item::ResponsesFunctionCallOutputItem;
use crate::responses_message_item::ResponsesMessageItem;
use crate::responses_reasoning_item::ResponsesReasoningItem;

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum ResponsesTaggedItem {
    #[serde(rename = "message")]
    Message(ResponsesMessageItem),
    #[serde(rename = "function_call")]
    FunctionCall(ResponsesFunctionCallItem),
    #[serde(rename = "function_call_output")]
    FunctionCallOutput(ResponsesFunctionCallOutputItem),
    #[serde(rename = "reasoning")]
    Reasoning(ResponsesReasoningItem),
}
