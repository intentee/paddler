use paddler_types::conversation_message::ConversationMessage;
use paddler_types::conversation_message_content::ConversationMessageContent;
use paddler_types::conversation_message_content_part::ConversationMessageContentPart;

pub fn create_text_only_conversation(
    conversation_history: &[ConversationMessage],
    media_marker: &str,
) -> Vec<ConversationMessage> {
    conversation_history
        .iter()
        .map(|message| ConversationMessage {
            content: match &message.content {
                ConversationMessageContent::Text(text) => {
                    ConversationMessageContent::Text(text.clone())
                }
                ConversationMessageContent::Parts(parts) => ConversationMessageContent::Parts(
                    parts
                        .iter()
                        .map(|part| match part {
                            ConversationMessageContentPart::Text { text } => {
                                ConversationMessageContentPart::Text { text: text.clone() }
                            }
                            ConversationMessageContentPart::ImageUrl { .. } => {
                                ConversationMessageContentPart::Text {
                                    text: media_marker.to_string(),
                                }
                            }
                        })
                        .collect(),
                ),
            },
            role: message.role.clone(),
        })
        .collect()
}
