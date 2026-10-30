use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::media_marker::MediaMarker;
use paddler_sha256_hex::sha256_lowercase_hex::sha256_lowercase_hex;

use crate::chat_template_tools::ChatTemplateTools;

#[must_use]
pub fn request_media_marker(
    conversation_history: &ConversationHistory,
    chat_template_tools: &ChatTemplateTools,
) -> MediaMarker {
    MediaMarker::new(format!(
        "<__media_{}__>",
        sha256_lowercase_hex(
            format!("{conversation_history:?}{}", chat_template_tools.json).as_bytes()
        )
    ))
}

#[cfg(test)]
mod tests {
    use serde_json::Value;
    use serde_json::json;

    use paddler_messaging::conversation_history::ConversationHistory;
    use paddler_messaging::conversation_message::ConversationMessage;
    use paddler_messaging::conversation_message_content::ConversationMessageContent;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::FunctionCall;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::function::Function;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters::Parameters;

    use super::request_media_marker;
    use crate::chat_template_tools::ChatTemplateTools;

    fn conversation_asking(question: &str) -> ConversationHistory {
        ConversationHistory::new(vec![ConversationMessage::Message {
            content: ConversationMessageContent::Text(question.to_owned()),
            role: "user".to_owned(),
        }])
    }

    fn tools_described_as(description: &str) -> ChatTemplateTools {
        ChatTemplateTools::new(&[Tool::<Value>::Function(FunctionCall {
            function: Function {
                name: "get_weather".to_owned(),
                description: description.to_owned(),
                parameters: Parameters::Schema(json!({"type": "object"})),
            },
        })])
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
