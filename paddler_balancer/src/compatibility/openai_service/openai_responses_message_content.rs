use paddler_messaging::conversation_message_content::ConversationMessageContent;
use serde::Deserialize;

use crate::compatibility::openai_service::openai_responses_input_content_part::OpenAIResponsesInputContentPart;

#[derive(Deserialize)]
#[serde(untagged)]
pub enum OpenAIResponsesMessageContent {
    Text(String),
    Parts(Vec<OpenAIResponsesInputContentPart>),
}

impl OpenAIResponsesMessageContent {
    #[must_use]
    pub fn into_conversation_content(self) -> ConversationMessageContent {
        match self {
            Self::Text(text) => ConversationMessageContent::Text(text),
            Self::Parts(parts) => ConversationMessageContent::Parts(
                parts
                    .into_iter()
                    .filter_map(OpenAIResponsesInputContentPart::into_conversation_part)
                    .collect(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::conversation_message_content::ConversationMessageContent;

    use super::OpenAIResponsesMessageContent;
    use crate::compatibility::openai_service::openai_responses_input_content_part::OpenAIResponsesInputContentPart;

    #[test]
    fn text_content_becomes_text() {
        assert!(matches!(
            OpenAIResponsesMessageContent::Text("hi".to_owned()).into_conversation_content(),
            ConversationMessageContent::Text(text) if text == "hi"
        ));
    }

    #[test]
    fn parts_content_becomes_parts_dropping_unsupported() {
        let content = OpenAIResponsesMessageContent::Parts(vec![
            OpenAIResponsesInputContentPart::InputText {
                text: "hello".to_owned(),
            },
            OpenAIResponsesInputContentPart::Unsupported,
        ]);

        assert!(matches!(
            content.into_conversation_content(),
            ConversationMessageContent::Parts(parts) if parts.len() == 1
        ));
    }
}
