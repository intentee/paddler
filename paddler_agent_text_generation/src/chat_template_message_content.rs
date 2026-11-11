use llama_cpp_bindings_types::chat_message_content::ChatMessageContent;
use llama_cpp_bindings_types::chat_message_content_part::ChatMessageContentPart;

use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::conversation_message_content_part::ConversationMessageContentPart;
use paddler_messaging::image_url::ImageUrl;

use crate::media_marker::MediaMarker;

pub struct ChatTemplateMessageContent(pub ConversationMessageContent);

impl ChatTemplateMessageContent {
    pub fn into_chat_message_content(
        self,
        MediaMarker { marker }: &MediaMarker,
        image_urls: &mut Vec<ImageUrl>,
    ) -> ChatMessageContent {
        match self.0 {
            ConversationMessageContent::Text(text) => ChatMessageContent::Text(text),
            ConversationMessageContent::Parts(parts) => ChatMessageContent::Parts(
                parts
                    .into_iter()
                    .map(|part| match part {
                        ConversationMessageContentPart::Text { text } => {
                            ChatMessageContentPart::Text(text)
                        }
                        ConversationMessageContentPart::ImageUrl { image_url } => {
                            image_urls.push(image_url);

                            ChatMessageContentPart::MediaMarker(marker.clone())
                        }
                    })
                    .collect(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings_types::chat_message_content::ChatMessageContent;
    use llama_cpp_bindings_types::chat_message_content_part::ChatMessageContentPart;

    use paddler_messaging::conversation_message_content::ConversationMessageContent;
    use paddler_messaging::conversation_message_content_part::ConversationMessageContentPart;
    use paddler_messaging::image_url::ImageUrl;

    use super::ChatTemplateMessageContent;
    use crate::media_marker::MediaMarker;

    #[test]
    fn images_reach_the_template_as_media_markers_between_their_text_parts() {
        let mut image_urls = Vec::new();
        let image_url = ImageUrl {
            url: "http://example.com/img.png".to_owned(),
        };
        let content = ConversationMessageContent::Parts(vec![
            ConversationMessageContentPart::Text {
                text: "before".to_owned(),
            },
            ConversationMessageContentPart::ImageUrl {
                image_url: image_url.clone(),
            },
            ConversationMessageContentPart::Text {
                text: "after".to_owned(),
            },
        ]);

        assert_eq!(
            ChatTemplateMessageContent(content).into_chat_message_content(
                &MediaMarker::new("[IMAGE]".to_owned()),
                &mut image_urls,
            ),
            ChatMessageContent::Parts(vec![
                ChatMessageContentPart::Text("before".to_owned()),
                ChatMessageContentPart::MediaMarker("[IMAGE]".to_owned()),
                ChatMessageContentPart::Text("after".to_owned()),
            ])
        );
        assert_eq!(image_urls, vec![image_url]);
    }
}
