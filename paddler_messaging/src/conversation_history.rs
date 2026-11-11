use serde::Deserialize;
use serde::Serialize;

use crate::conversation_message::ConversationMessage;

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
    pub fn contains_image_url(&self) -> bool {
        self.messages
            .iter()
            .any(ConversationMessage::contains_image_url)
    }
}

#[cfg(test)]
mod tests {
    use super::ConversationHistory;
    use crate::conversation_message::ConversationMessage;
    use crate::conversation_message_content::ConversationMessageContent;
    use crate::conversation_message_content_part::ConversationMessageContentPart;
    use crate::image_url::ImageUrl;

    #[test]
    fn a_conversation_of_reasoning_and_text_contains_no_image_url() {
        let conversation_history = ConversationHistory::new(vec![
            ConversationMessage::AssistantReasoning {
                reasoning_content: "thinking".to_owned(),
            },
            ConversationMessage::Message {
                content: ConversationMessageContent::Text("hello".to_owned()),
                role: "user".to_owned(),
            },
        ]);

        assert!(!conversation_history.contains_image_url());
    }

    #[test]
    fn a_conversation_with_an_image_in_a_tool_result_contains_an_image_url() {
        let conversation_history =
            ConversationHistory::new(vec![ConversationMessage::ToolResult {
                content: ConversationMessageContent::Parts(vec![
                    ConversationMessageContentPart::ImageUrl {
                        image_url: ImageUrl {
                            url: "data:image/png;base64,AAAA".to_owned(),
                        },
                    },
                ]),
                tool_call_id: "call_weather".to_owned(),
            }]);

        assert!(conversation_history.contains_image_url());
    }
}
