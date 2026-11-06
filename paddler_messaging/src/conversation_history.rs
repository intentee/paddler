use serde::Deserialize;
use serde::Serialize;

use crate::chat_template_conversation::ChatTemplateConversation;
use crate::conversation_message::ConversationMessage;
use crate::media_marker::MediaMarker;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct ConversationHistory {
    pub messages: Vec<ConversationMessage>,
}

impl ConversationHistory {
    #[must_use]
    pub const fn new(messages: Vec<ConversationMessage>) -> Self {
        Self { messages }
    }

    #[must_use]
    pub fn into_chat_template_conversation(
        self,
        media_marker: &MediaMarker,
    ) -> ChatTemplateConversation {
        let mut image_urls = Vec::new();
        let mut messages = Vec::with_capacity(self.messages.len());
        let mut conversation_messages = self.messages.into_iter().peekable();

        while let Some(conversation_message) = conversation_messages.next() {
            messages.push(match conversation_message {
                ConversationMessage::AssistantReasoning { reasoning_content } => {
                    match conversation_messages.next_if(ConversationMessage::is_assistant_reply) {
                        Some(assistant_reply) => assistant_reply.into_chat_template_message(
                            media_marker,
                            &mut image_urls,
                            Some(reasoning_content),
                        ),
                        None => ConversationMessage::AssistantReasoning { reasoning_content }
                            .into_chat_template_message(media_marker, &mut image_urls, None),
                    }
                }
                unreasoned_message => unreasoned_message.into_chat_template_message(
                    media_marker,
                    &mut image_urls,
                    None,
                ),
            });
        }

        ChatTemplateConversation {
            image_urls,
            messages,
        }
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings_types::chat_message::ChatMessage;
    use llama_cpp_bindings_types::chat_message_content::ChatMessageContent;
    use llama_cpp_bindings_types::chat_message_content_part::ChatMessageContentPart;
    use llama_cpp_bindings_types::chat_message_tool_call::ChatMessageToolCall;
    use serde_json::json;

    use super::ConversationHistory;
    use crate::chat_template_conversation::ChatTemplateConversation;
    use crate::conversation_message::ConversationMessage;
    use crate::conversation_message_content::ConversationMessageContent;
    use crate::conversation_message_content_part::ConversationMessageContentPart;
    use crate::conversation_tool_call::ConversationToolCall;
    use crate::image_url::ImageUrl;
    use crate::media_marker::MediaMarker;

    fn make_text_message(role: &str, text: &str) -> ConversationMessage {
        ConversationMessage::Message {
            content: ConversationMessageContent::Text(text.to_owned()),
            role: role.to_owned(),
        }
    }

    fn weather_tool_call() -> ConversationToolCall {
        ConversationToolCall {
            arguments: json!({ "location": "Paris" }).as_object().unwrap().clone(),
            id: "call_weather".to_owned(),
            name: "get_weather".to_owned(),
        }
    }

    fn template_weather_tool_call() -> ChatMessageToolCall {
        ChatMessageToolCall {
            id: "call_weather".to_owned(),
            name: "get_weather".to_owned(),
            arguments: r#"{"location":"Paris"}"#.to_owned(),
        }
    }

    fn template_messages(history: ConversationHistory) -> Vec<ChatMessage> {
        history
            .into_chat_template_conversation(&MediaMarker::new("[IMAGE]".to_owned()))
            .messages
    }

    #[test]
    fn collects_image_urls_and_replaces_images_with_the_media_marker() {
        let history = ConversationHistory::new(vec![
            make_text_message("assistant", "hello"),
            ConversationMessage::Message {
                content: ConversationMessageContent::Parts(vec![
                    ConversationMessageContentPart::Text {
                        text: "before".to_owned(),
                    },
                    ConversationMessageContentPart::ImageUrl {
                        image_url: ImageUrl {
                            url: "http://example.com/img.png".to_owned(),
                        },
                    },
                    ConversationMessageContentPart::Text {
                        text: "after".to_owned(),
                    },
                ]),
                role: "user".to_owned(),
            },
        ]);

        assert_eq!(
            history.into_chat_template_conversation(&MediaMarker::new("[IMAGE]".to_owned())),
            ChatTemplateConversation {
                image_urls: vec![ImageUrl {
                    url: "http://example.com/img.png".to_owned(),
                }],
                messages: vec![
                    ChatMessage::Message {
                        role: "assistant".to_owned(),
                        content: ChatMessageContent::Text("hello".to_owned()),
                        reasoning_content: None,
                    },
                    ChatMessage::Message {
                        role: "user".to_owned(),
                        content: ChatMessageContent::Parts(vec![
                            ChatMessageContentPart::Text("before".to_owned()),
                            ChatMessageContentPart::MediaMarker("[IMAGE]".to_owned()),
                            ChatMessageContentPart::Text("after".to_owned()),
                        ]),
                        reasoning_content: None,
                    },
                ],
            }
        );
    }

    #[test]
    fn an_assistant_tool_call_turn_reaches_the_template_with_structured_tool_calls() {
        let history = ConversationHistory::new(vec![ConversationMessage::AssistantToolCalls {
            content: ConversationMessageContent::Text(String::new()),
            tool_calls: vec![weather_tool_call()],
        }]);

        assert_eq!(
            template_messages(history),
            vec![ChatMessage::AssistantToolCalls {
                content: ChatMessageContent::Text(String::new()),
                reasoning_content: None,
                tool_calls: vec![template_weather_tool_call()],
            }]
        );
    }

    #[test]
    fn a_tool_result_reaches_the_template_with_its_tool_call_id() {
        let history = ConversationHistory::new(vec![ConversationMessage::ToolResult {
            content: ConversationMessageContent::Text("sunny".to_owned()),
            tool_call_id: "call_weather".to_owned(),
        }]);

        assert_eq!(
            template_messages(history),
            vec![ChatMessage::ToolResult {
                content: ChatMessageContent::Text("sunny".to_owned()),
                tool_call_id: "call_weather".to_owned(),
            }]
        );
    }

    #[test]
    fn a_reasoning_reaches_the_template_on_the_assistant_reply_after_it() {
        let history = ConversationHistory::new(vec![
            ConversationMessage::AssistantReasoning {
                reasoning_content: "the user wants the weather".to_owned(),
            },
            ConversationMessage::AssistantToolCalls {
                content: ConversationMessageContent::Text(String::new()),
                tool_calls: vec![weather_tool_call()],
            },
        ]);

        assert_eq!(
            template_messages(history),
            vec![ChatMessage::AssistantToolCalls {
                content: ChatMessageContent::Text(String::new()),
                reasoning_content: Some("the user wants the weather".to_owned()),
                tool_calls: vec![template_weather_tool_call()],
            }]
        );
    }

    #[test]
    fn a_reasoning_no_assistant_reply_follows_reaches_the_template_as_its_own_assistant_turn() {
        let history = ConversationHistory::new(vec![
            ConversationMessage::AssistantReasoning {
                reasoning_content: "nothing to answer yet".to_owned(),
            },
            make_text_message("user", "hello"),
        ]);

        assert_eq!(
            template_messages(history),
            vec![
                ChatMessage::Message {
                    role: "assistant".to_owned(),
                    content: ChatMessageContent::Text(String::new()),
                    reasoning_content: Some("nothing to answer yet".to_owned()),
                },
                ChatMessage::Message {
                    role: "user".to_owned(),
                    content: ChatMessageContent::Text("hello".to_owned()),
                    reasoning_content: None,
                },
            ]
        );
    }

    #[test]
    fn a_reasoning_followed_by_a_tool_result_is_not_attached_to_that_tool_result() {
        let history = ConversationHistory::new(vec![
            ConversationMessage::AssistantReasoning {
                reasoning_content: "waiting for the weather".to_owned(),
            },
            ConversationMessage::ToolResult {
                content: ConversationMessageContent::Text("sunny".to_owned()),
                tool_call_id: "call_weather".to_owned(),
            },
        ]);

        assert_eq!(
            template_messages(history),
            vec![
                ChatMessage::Message {
                    role: "assistant".to_owned(),
                    content: ChatMessageContent::Text(String::new()),
                    reasoning_content: Some("waiting for the weather".to_owned()),
                },
                ChatMessage::ToolResult {
                    content: ChatMessageContent::Text("sunny".to_owned()),
                    tool_call_id: "call_weather".to_owned(),
                },
            ]
        );
    }
}
