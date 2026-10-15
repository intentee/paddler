use serde::Deserialize;
use serde_json::json;

use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;

use crate::assistant_role::ASSISTANT_ROLE;
use crate::responses_function_call_item::ResponsesFunctionCallItem;
use crate::responses_function_call_output_item::ResponsesFunctionCallOutputItem;
use crate::responses_message_item::ResponsesMessageItem;
use crate::responses_tagged_item::ResponsesTaggedItem;

#[derive(Deserialize)]
#[serde(untagged)]
pub enum ResponsesInputItem {
    Tagged(ResponsesTaggedItem),
    Message(ResponsesMessageItem),
}

impl ResponsesInputItem {
    #[must_use]
    pub fn into_conversation_message(self) -> ConversationMessage {
        match self {
            Self::Message(message) | Self::Tagged(ResponsesTaggedItem::Message(message)) => {
                message.into_conversation_message()
            }
            Self::Tagged(ResponsesTaggedItem::FunctionCall(ResponsesFunctionCallItem {
                call_id,
                name,
                arguments,
            })) => ConversationMessage {
                content: ConversationMessageContent::Text(
                    json!({ "call_id": call_id, "name": name, "arguments": arguments }).to_string(),
                ),
                role: ASSISTANT_ROLE.to_owned(),
            },
            Self::Tagged(ResponsesTaggedItem::FunctionCallOutput(
                ResponsesFunctionCallOutputItem { output },
            )) => ConversationMessage {
                content: ConversationMessageContent::Text(output.into_text()),
                role: "tool".to_owned(),
            },
        }
    }
}
