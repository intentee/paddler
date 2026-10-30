use serde::Deserialize;

use paddler_messaging::assistant_role::ASSISTANT_ROLE;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::system_role::SYSTEM_ROLE;
use paddler_messaging::user_role::USER_ROLE;

use crate::chat_completion_message_role::ChatCompletionMessageRole;
use crate::chat_completion_requested_tool_call::ChatCompletionRequestedToolCall;
use crate::openai_translation_error::OpenAITranslationError;

#[derive(Deserialize)]
pub struct ChatCompletionMessage {
    pub content: Option<ConversationMessageContent>,
    pub reasoning_content: Option<String>,
    pub role: ChatCompletionMessageRole,
    pub tool_call_id: Option<String>,
    pub tool_calls: Option<Vec<ChatCompletionRequestedToolCall>>,
}

impl ChatCompletionMessage {
    pub fn append_to(
        self,
        messages: &mut Vec<ConversationMessage>,
    ) -> Result<(), OpenAITranslationError> {
        let Self {
            content,
            reasoning_content,
            role,
            tool_call_id,
            tool_calls,
        } = self;
        let assistant_reasoning = reasoning_content
            .map(|reasoning_content| match role {
                ChatCompletionMessageRole::Assistant => {
                    Ok(ConversationMessage::AssistantReasoning { reasoning_content })
                }
                ChatCompletionMessageRole::Developer
                | ChatCompletionMessageRole::System
                | ChatCompletionMessageRole::Tool
                | ChatCompletionMessageRole::User => {
                    Err(OpenAITranslationError::ReasoningOnNonAssistantMessage { role })
                }
            })
            .transpose()?;
        let reply = match tool_calls {
            Some(tool_calls) if !tool_calls.is_empty() => {
                if role != ChatCompletionMessageRole::Assistant {
                    return Err(OpenAITranslationError::ToolCallsOnNonAssistantMessage { role });
                }

                ConversationMessage::AssistantToolCalls {
                    content: content
                        .unwrap_or_else(|| ConversationMessageContent::Text(String::new())),
                    tool_calls: tool_calls
                        .into_iter()
                        .map(ChatCompletionRequestedToolCall::into_conversation_tool_call)
                        .collect::<Result<_, _>>()?,
                }
            }
            Some(_) | None => {
                let Some(content) = content else {
                    return Err(OpenAITranslationError::MessageWithoutContent { role });
                };

                match role {
                    ChatCompletionMessageRole::Assistant => ConversationMessage::Message {
                        content,
                        role: ASSISTANT_ROLE.to_owned(),
                    },
                    ChatCompletionMessageRole::Developer | ChatCompletionMessageRole::System => {
                        ConversationMessage::Message {
                            content,
                            role: SYSTEM_ROLE.to_owned(),
                        }
                    }
                    ChatCompletionMessageRole::Tool => ConversationMessage::ToolResult {
                        content,
                        tool_call_id: tool_call_id
                            .ok_or(OpenAITranslationError::ToolResultWithoutToolCallId)?,
                    },
                    ChatCompletionMessageRole::User => ConversationMessage::Message {
                        content,
                        role: USER_ROLE.to_owned(),
                    },
                }
            }
        };

        messages.extend(assistant_reasoning);
        messages.push(reply);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;
    use serde_json::from_value;
    use serde_json::json;

    use paddler_messaging::conversation_message::ConversationMessage;
    use paddler_messaging::conversation_message_content::ConversationMessageContent;
    use paddler_messaging::conversation_message_content_part::ConversationMessageContentPart;
    use paddler_messaging::conversation_tool_call::ConversationToolCall;
    use paddler_messaging::image_url::ImageUrl;

    use super::ChatCompletionMessage;
    use crate::chat_completion_message_role::ChatCompletionMessageRole;
    use crate::openai_translation_error::OpenAITranslationError;

    fn appended(message: Value) -> Result<Vec<ConversationMessage>, OpenAITranslationError> {
        let mut messages = Vec::new();

        from_value::<ChatCompletionMessage>(message)
            .unwrap()
            .append_to(&mut messages)
            .map(|()| messages)
    }

    fn converted(message: Value) -> Result<ConversationMessage, OpenAITranslationError> {
        appended(message).map(|mut messages| messages.remove(0))
    }

    fn read_call(id: &str, path: &str) -> ConversationToolCall {
        ConversationToolCall {
            arguments: json!({ "path": path }).as_object().unwrap().clone(),
            id: id.to_owned(),
            name: "read".to_owned(),
        }
    }

    #[test]
    fn a_user_message_with_parts_keeps_its_role_and_parts() {
        assert_eq!(
            converted(json!({
                "role": "user",
                "content": [
                    {"type": "text", "text": "OCR this"},
                    {"type": "image_url", "image_url": {"url": "data:image/png;base64,abc"}}
                ]
            }))
            .unwrap(),
            ConversationMessage::Message {
                content: ConversationMessageContent::Parts(vec![
                    ConversationMessageContentPart::Text {
                        text: "OCR this".to_owned(),
                    },
                    ConversationMessageContentPart::ImageUrl {
                        image_url: ImageUrl {
                            url: "data:image/png;base64,abc".to_owned(),
                        },
                    },
                ]),
                role: "user".to_owned(),
            }
        );
    }

    #[test]
    fn parallel_tool_calls_without_content_become_one_assistant_turn_with_empty_text() {
        assert_eq!(
            converted(json!({
                "role": "assistant",
                "content": null,
                "tool_calls": [
                    {"id": "call_0", "type": "function", "function": {"name": "read", "arguments": "{\"path\":\"a.txt\"}"}},
                    {"id": "call_1", "type": "function", "function": {"name": "read", "arguments": "{\"path\":\"b.txt\"}"}}
                ]
            }))
            .unwrap(),
            ConversationMessage::AssistantToolCalls {
                content: ConversationMessageContent::Text(String::new()),
                tool_calls: vec![read_call("call_0", "a.txt"), read_call("call_1", "b.txt")],
            }
        );
    }

    #[test]
    fn assistant_text_alongside_tool_calls_stays_in_the_same_turn() {
        assert_eq!(
            converted(json!({
                "role": "assistant",
                "content": "Let me read it.",
                "tool_calls": [
                    {"id": "call_0", "type": "function", "function": {"name": "read", "arguments": "{\"path\":\"a.txt\"}"}}
                ]
            }))
            .unwrap(),
            ConversationMessage::AssistantToolCalls {
                content: ConversationMessageContent::Text("Let me read it.".to_owned()),
                tool_calls: vec![read_call("call_0", "a.txt")],
            }
        );
    }

    #[test]
    fn a_tool_message_keeps_its_tool_call_id() {
        assert_eq!(
            converted(
                json!({"role": "tool", "tool_call_id": "call_0", "content": "file contents"})
            )
            .unwrap(),
            ConversationMessage::ToolResult {
                content: ConversationMessageContent::Text("file contents".to_owned()),
                tool_call_id: "call_0".to_owned(),
            }
        );
    }

    #[test]
    fn a_message_without_content_or_tool_calls_is_refused_naming_its_role() {
        assert_eq!(
            converted(json!({"role": "assistant", "content": null}))
                .err()
                .map(|translation_error| translation_error.to_string()),
            Some(
                OpenAITranslationError::MessageWithoutContent {
                    role: ChatCompletionMessageRole::Assistant
                }
                .to_string()
            )
        );
    }

    #[test]
    fn tool_calls_on_a_message_that_is_not_an_assistant_message_are_refused() {
        assert_eq!(
            converted(json!({
                "role": "user",
                "content": "hi",
                "tool_calls": [
                    {"id": "call_0", "type": "function", "function": {"name": "read", "arguments": "{}"}}
                ]
            }))
            .err()
            .map(|translation_error| translation_error.to_string()),
            Some(
                OpenAITranslationError::ToolCallsOnNonAssistantMessage {
                    role: ChatCompletionMessageRole::User
                }
                .to_string()
            )
        );
    }

    #[test]
    fn a_tool_message_without_a_tool_call_id_is_refused() {
        assert_eq!(
            converted(json!({"role": "tool", "content": "file contents"}))
                .err()
                .as_ref()
                .map(ToString::to_string),
            Some(OpenAITranslationError::ToolResultWithoutToolCallId.to_string())
        );
    }

    #[test]
    fn tool_call_arguments_that_are_not_a_json_object_are_refused() {
        assert!(matches!(
            converted(json!({
                "role": "assistant",
                "content": null,
                "tool_calls": [
                    {"id": "call_0", "type": "function", "function": {"name": "read", "arguments": "[\"a.txt\"]"}}
                ]
            })),
            Err(OpenAITranslationError::ToolCallArgumentsAreNotAJsonObject { tool_call_id, .. })
                if tool_call_id == "call_0"
        ));
    }

    #[test]
    fn a_developer_message_instructs_the_model_as_a_system_message() {
        assert_eq!(
            converted(json!({"role": "developer", "content": "Answer in French"})).unwrap(),
            ConversationMessage::Message {
                content: ConversationMessageContent::Text("Answer in French".to_owned()),
                role: "system".to_owned(),
            }
        );
    }

    #[test]
    fn a_message_of_a_role_openai_does_not_define_is_refused() {
        assert!(
            from_value::<ChatCompletionMessage>(json!({"role": "ipython", "content": "1"}))
                .is_err()
        );
    }

    #[test]
    fn the_reasoning_of_an_assistant_message_precedes_its_reply() {
        assert_eq!(
            appended(json!({
                "role": "assistant",
                "content": "It is sunny.",
                "reasoning_content": "The forecast said sun."
            }))
            .unwrap(),
            vec![
                ConversationMessage::AssistantReasoning {
                    reasoning_content: "The forecast said sun.".to_owned(),
                },
                ConversationMessage::Message {
                    content: ConversationMessageContent::Text("It is sunny.".to_owned()),
                    role: "assistant".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn reasoning_on_a_message_that_is_not_an_assistant_message_is_refused() {
        assert_eq!(
            appended(json!({
                "role": "user",
                "content": "hi",
                "reasoning_content": "I am thinking"
            }))
            .err()
            .map(|translation_error| translation_error.to_string()),
            Some(
                OpenAITranslationError::ReasoningOnNonAssistantMessage {
                    role: ChatCompletionMessageRole::User
                }
                .to_string()
            )
        );
    }
}
