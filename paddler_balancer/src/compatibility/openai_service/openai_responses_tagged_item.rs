use serde::Deserialize;

use crate::compatibility::openai_service::openai_responses_function_call_item::OpenAIResponsesFunctionCallItem;
use crate::compatibility::openai_service::openai_responses_function_call_output_item::OpenAIResponsesFunctionCallOutputItem;
use crate::compatibility::openai_service::openai_responses_message_item::OpenAIResponsesMessageItem;

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum OpenAIResponsesTaggedItem {
    #[serde(rename = "message")]
    Message(OpenAIResponsesMessageItem),
    #[serde(rename = "function_call")]
    FunctionCall(OpenAIResponsesFunctionCallItem),
    #[serde(rename = "function_call_output")]
    FunctionCallOutput(OpenAIResponsesFunctionCallOutputItem),
    #[serde(other)]
    Unsupported,
}
