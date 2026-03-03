use paddler_types::conversation_message::ConversationMessage;
use paddler_types::conversation_message_content::ConversationMessageContent;

pub fn create_text_only_conversation(
    conversation_history: &[ConversationMessage],
    media_marker: &str,
) -> Vec<ConversationMessage> {
    conversation_history
        .iter()
        .map(|message| ConversationMessage {
            content: ConversationMessageContent::Text(
                message
                    .content
                    .text_content_with_media_markers(media_marker),
            ),
            role: message.role.clone(),
        })
        .collect()
}
