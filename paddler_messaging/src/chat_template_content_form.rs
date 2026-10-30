use crate::chat_template_message_content::ChatTemplateMessageContent;
use crate::chat_template_message_content_part::ChatTemplateMessageContentPart;

const TEXT_CONTENT_PART_TYPE: &str = "text";
const TEXT_PARTS_SEPARATOR: &str = "\n";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChatTemplateContentForm {
    JoinedText,
    TypedParts,
}

impl ChatTemplateContentForm {
    #[must_use]
    pub fn content_of(self, texts: Vec<String>) -> ChatTemplateMessageContent {
        match self {
            Self::JoinedText => ChatTemplateMessageContent::Text(texts.join(TEXT_PARTS_SEPARATOR)),
            Self::TypedParts => ChatTemplateMessageContent::Parts(
                texts
                    .into_iter()
                    .map(|text| ChatTemplateMessageContentPart {
                        content_type: TEXT_CONTENT_PART_TYPE,
                        text,
                    })
                    .collect(),
            ),
        }
    }
}
