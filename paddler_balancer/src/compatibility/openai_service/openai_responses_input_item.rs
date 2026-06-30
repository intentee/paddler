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

#[cfg(test)]
mod tests {
    use paddler_messaging::conversation_message_content::ConversationMessageContent;

    use super::OpenAIResponsesInputItem;
    use crate::compatibility::openai_service::openai_responses_function_call_item::OpenAIResponsesFunctionCallItem;
    use crate::compatibility::openai_service::openai_responses_function_call_output_item::OpenAIResponsesFunctionCallOutputItem;
    use crate::compatibility::openai_service::openai_responses_function_output::OpenAIResponsesFunctionOutput;
    use crate::compatibility::openai_service::openai_responses_message_content::OpenAIResponsesMessageContent;
    use crate::compatibility::openai_service::openai_responses_message_item::OpenAIResponsesMessageItem;
    use crate::compatibility::openai_service::openai_responses_tagged_item::OpenAIResponsesTaggedItem;

    fn text_message_item(role: &str, text: &str) -> OpenAIResponsesMessageItem {
        OpenAIResponsesMessageItem {
            role: role.to_owned(),
            content: OpenAIResponsesMessageContent::Text(text.to_owned()),
        }
    }

    #[test]
    fn direct_message_becomes_a_conversation_message() {
        let message = OpenAIResponsesInputItem::Message(text_message_item("user", "hi"))
            .into_conversation_message()
            .expect("a direct message item produces a conversation message");

        assert_eq!(message.role, "user");
        assert!(matches!(
            message.content,
            ConversationMessageContent::Text(text) if text == "hi"
        ));
    }

    #[test]
    fn tagged_message_becomes_a_conversation_message() {
        let message = OpenAIResponsesInputItem::Tagged(OpenAIResponsesTaggedItem::Message(
            text_message_item("user", "tagged hi"),
        ))
        .into_conversation_message()
        .expect("a tagged message item produces a conversation message");

        assert_eq!(message.role, "user");
        assert!(matches!(
            message.content,
            ConversationMessageContent::Text(text) if text == "tagged hi"
        ));
    }

    #[test]
    fn function_call_becomes_an_assistant_message_with_serialized_call() {
        let message = OpenAIResponsesInputItem::Tagged(OpenAIResponsesTaggedItem::FunctionCall(
            OpenAIResponsesFunctionCallItem {
                call_id: "call_1".to_owned(),
                name: "get_weather".to_owned(),
                arguments: "{\"city\":\"Paris\"}".to_owned(),
            },
        ))
        .into_conversation_message()
        .expect("a function call item produces a conversation message");

        assert_eq!(message.role, "assistant");
        assert!(matches!(
            message.content,
            ConversationMessageContent::Text(text)
                if text.contains("call_1") && text.contains("get_weather")
        ));
    }

    #[test]
    fn function_call_output_becomes_a_tool_message() {
        let message = OpenAIResponsesInputItem::Tagged(
            OpenAIResponsesTaggedItem::FunctionCallOutput(OpenAIResponsesFunctionCallOutputItem {
                output: OpenAIResponsesFunctionOutput::Text("sunny".to_owned()),
            }),
        )
        .into_conversation_message()
        .expect("a function call output item produces a conversation message");

        assert_eq!(message.role, "tool");
        assert!(matches!(
            message.content,
            ConversationMessageContent::Text(text) if text == "sunny"
        ));
    }

    #[test]
    fn unsupported_tagged_item_is_dropped() {
        assert!(
            OpenAIResponsesInputItem::Tagged(OpenAIResponsesTaggedItem::Unsupported)
                .into_conversation_message()
                .is_none()
        );
    }
}
