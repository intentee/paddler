use llama_cpp_bindings_types::chat_message_content::ChatMessageContent;
use llama_cpp_bindings_types::chat_message_content_part::ChatMessageContentPart;

use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::conversation_message_content_part::ConversationMessageContentPart;

use crate::conversation_media::ConversationMedia;
use crate::media_marker::MediaMarker;

pub struct ChatTemplateMessageContent(pub ConversationMessageContent);

impl ChatTemplateMessageContent {
    pub fn into_chat_message_content(
        self,
        MediaMarker { marker }: &MediaMarker,
        media: &mut Vec<ConversationMedia>,
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
                            media.push(ConversationMedia::Image(image_url));

                            ChatMessageContentPart::MediaMarker(marker.clone())
                        }
                        ConversationMessageContentPart::InputAudio { input_audio } => {
                            media.push(ConversationMedia::Audio(input_audio));

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

    use paddler_messaging::audio_format::AudioFormat;
    use paddler_messaging::conversation_message_content::ConversationMessageContent;
    use paddler_messaging::conversation_message_content_part::ConversationMessageContentPart;
    use paddler_messaging::image_url::ImageUrl;
    use paddler_messaging::input_audio::InputAudio;

    use super::ChatTemplateMessageContent;
    use crate::conversation_media::ConversationMedia;
    use crate::media_marker::MediaMarker;

    #[test]
    fn media_reach_the_template_as_media_markers_between_their_text_parts() {
        let mut media = Vec::new();
        let image_url = ImageUrl {
            url: "http://example.com/img.png".to_owned(),
        };
        let input_audio = InputAudio {
            data: "UklGRg==".to_owned(),
            format: AudioFormat::Wav,
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
            ConversationMessageContentPart::InputAudio {
                input_audio: input_audio.clone(),
            },
        ]);

        assert_eq!(
            ChatTemplateMessageContent(content)
                .into_chat_message_content(&MediaMarker::new("[MEDIA]".to_owned()), &mut media,),
            ChatMessageContent::Parts(vec![
                ChatMessageContentPart::Text("before".to_owned()),
                ChatMessageContentPart::MediaMarker("[MEDIA]".to_owned()),
                ChatMessageContentPart::Text("after".to_owned()),
                ChatMessageContentPart::MediaMarker("[MEDIA]".to_owned()),
            ])
        );
        assert_eq!(
            media,
            vec![
                ConversationMedia::Image(image_url),
                ConversationMedia::Audio(input_audio)
            ]
        );
    }
}
