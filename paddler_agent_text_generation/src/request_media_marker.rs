use llama_cpp_bindings_types::chat_tool::ChatTool;

use paddler_messaging::conversation_history::ConversationHistory;
use paddler_sha256_hex::sha256_lowercase_hex::sha256_lowercase_hex;

use crate::media_marker::MediaMarker;

const TEXT_ONLY_MEDIA_MARKER: &str = "<__media__>";

#[must_use]
pub fn request_media_marker(
    conversation_history: &ConversationHistory,
    tools: &[ChatTool],
) -> MediaMarker {
    if !conversation_history.contains_image_url() {
        return MediaMarker::new(TEXT_ONLY_MEDIA_MARKER.to_owned());
    }

    MediaMarker::new(format!(
        "<__media_{}__>",
        sha256_lowercase_hex(format!("{conversation_history:?}{tools:?}").as_bytes())
    ))
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings_types::chat_tool::ChatTool;
    use serde_json::json;

    use paddler_messaging::conversation_history::ConversationHistory;
    use paddler_messaging::conversation_message::ConversationMessage;
    use paddler_messaging::conversation_message_content::ConversationMessageContent;
    use paddler_messaging::conversation_message_content_part::ConversationMessageContentPart;
    use paddler_messaging::image_url::ImageUrl;

    use super::TEXT_ONLY_MEDIA_MARKER;
    use super::request_media_marker;

    fn conversation_asking_about_an_image(question: &str) -> ConversationHistory {
        ConversationHistory::new(vec![ConversationMessage::Message {
            content: ConversationMessageContent::Parts(vec![
                ConversationMessageContentPart::Text {
                    text: question.to_owned(),
                },
                ConversationMessageContentPart::ImageUrl {
                    image_url: ImageUrl {
                        url: "data:image/png;base64,AAAA".to_owned(),
                    },
                },
            ]),
            role: "user".to_owned(),
        }])
    }

    fn tools_described_as(description: &str) -> Vec<ChatTool> {
        vec![ChatTool {
            name: "get_weather".to_owned(),
            description: description.to_owned(),
            parameters: Some(json!({"type": "object"})),
        }]
    }

    #[test]
    fn a_text_only_request_gets_the_text_only_marker_without_hashing_its_conversation() {
        let conversation_history = ConversationHistory::new(vec![ConversationMessage::Message {
            content: ConversationMessageContent::Text("Describe the weather".to_owned()),
            role: "user".to_owned(),
        }]);

        assert_eq!(
            request_media_marker(
                &conversation_history,
                &tools_described_as("Reports the weather")
            )
            .marker,
            TEXT_ONLY_MEDIA_MARKER
        );
    }

    #[test]
    fn requests_asking_different_questions_get_different_markers() {
        let tools = tools_described_as("Reports the weather");

        assert_ne!(
            request_media_marker(
                &conversation_asking_about_an_image("Describe the image"),
                &tools
            )
            .marker,
            request_media_marker(
                &conversation_asking_about_an_image("Count the llamas"),
                &tools
            )
            .marker
        );
    }

    #[test]
    fn requests_describing_their_tools_differently_get_different_markers() {
        let conversation = conversation_asking_about_an_image("Describe the image");

        assert_ne!(
            request_media_marker(&conversation, &tools_described_as("Reports the weather")).marker,
            request_media_marker(&conversation, &tools_described_as("Reports the forecast")).marker
        );
    }
}
