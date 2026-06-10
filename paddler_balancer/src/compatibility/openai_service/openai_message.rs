use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct OpenAIMessage {
    pub content: ConversationMessageContent,
    pub role: String,
}

impl OpenAIMessage {
    #[must_use]
    pub fn to_conversation_message(&self) -> ConversationMessage {
        ConversationMessage {
            content: self.content.clone(),
            role: self.role.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::OpenAIMessage;

    #[test]
    fn openai_message_converts_to_conversation_message() {
        let input = json!({
            "role": "user",
            "content": [
                {"type": "text", "text": "OCR this"},
                {"type": "image_url", "image_url": {"url": "data:image/png;base64,abc"}}
            ]
        });

        let openai_message: OpenAIMessage = serde_json::from_value(input).unwrap();
        let conversation_message = openai_message.to_conversation_message();

        assert_eq!(conversation_message.role, "user");
        assert_eq!(conversation_message.content.text_content(), "OCR this");
        assert_eq!(conversation_message.content.image_urls().len(), 1);
    }
}
