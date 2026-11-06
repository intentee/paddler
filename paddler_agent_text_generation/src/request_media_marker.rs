use llama_cpp_bindings_types::chat_tool::ChatTool;

use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::media_marker::MediaMarker;
use paddler_sha256_hex::sha256_lowercase_hex::sha256_lowercase_hex;

#[must_use]
pub fn request_media_marker(
    conversation_history: &ConversationHistory,
    tools: &[ChatTool],
) -> MediaMarker {
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

    use super::request_media_marker;

    fn conversation_asking(question: &str) -> ConversationHistory {
        ConversationHistory::new(vec![ConversationMessage::Message {
            content: ConversationMessageContent::Text(question.to_owned()),
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
    fn requests_asking_different_questions_get_different_markers() {
        let tools = tools_described_as("Reports the weather");

        assert_ne!(
            request_media_marker(&conversation_asking("Describe the image"), &tools).marker,
            request_media_marker(&conversation_asking("Count the llamas"), &tools).marker
        );
    }

    #[test]
    fn requests_describing_their_tools_differently_get_different_markers() {
        let conversation = conversation_asking("Describe the image");

        assert_ne!(
            request_media_marker(&conversation, &tools_described_as("Reports the weather")).marker,
            request_media_marker(&conversation, &tools_described_as("Reports the forecast")).marker
        );
    }
}
