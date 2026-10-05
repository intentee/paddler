use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use serde::Deserialize;
use serde_json::json;

use crate::compatibility::openai_service::openai_responses_function_call_item::OpenAIResponsesFunctionCallItem;
use crate::compatibility::openai_service::openai_responses_function_call_output_item::OpenAIResponsesFunctionCallOutputItem;
use crate::compatibility::openai_service::openai_responses_message_item::OpenAIResponsesMessageItem;
use crate::compatibility::openai_service::openai_responses_tagged_item::OpenAIResponsesTaggedItem;

#[derive(Deserialize)]
#[serde(untagged)]
pub enum OpenAIResponsesInputItem {
    Tagged(OpenAIResponsesTaggedItem),
    Message(OpenAIResponsesMessageItem),
}

impl OpenAIResponsesInputItem {
    #[must_use]
    pub fn into_conversation_message(self) -> Option<ConversationMessage> {
        match self {
            Self::Message(message) | Self::Tagged(OpenAIResponsesTaggedItem::Message(message)) => {
                Some(message.into_conversation_message())
            }
            Self::Tagged(OpenAIResponsesTaggedItem::FunctionCall(
                OpenAIResponsesFunctionCallItem {
                    call_id,
                    name,
                    arguments,
                },
            )) => Some(ConversationMessage {
                content: ConversationMessageContent::Text(
                    json!({ "call_id": call_id, "name": name, "arguments": arguments }).to_string(),
                ),
                role: "assistant".to_owned(),
            }),
            Self::Tagged(OpenAIResponsesTaggedItem::FunctionCallOutput(
                OpenAIResponsesFunctionCallOutputItem { output },
            )) => Some(ConversationMessage {
                content: ConversationMessageContent::Text(output.into_text()),
                role: "tool".to_owned(),
            }),
            Self::Tagged(OpenAIResponsesTaggedItem::Unsupported) => None,
        }
    }
}
