use llama_cpp_bindings_types::chat_message_tool_call::ChatMessageToolCall;
use serde_json::Value;

use paddler_messaging::conversation_tool_call::ConversationToolCall;

pub struct ChatTemplateToolCall(pub ConversationToolCall);

impl ChatTemplateToolCall {
    #[must_use]
    pub fn into_chat_message_tool_call(self) -> ChatMessageToolCall {
        let Self(ConversationToolCall {
            arguments,
            id,
            name,
        }) = self;

        ChatMessageToolCall {
            id,
            name,
            arguments: Value::Object(arguments).to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings_types::chat_message_tool_call::ChatMessageToolCall;
    use serde_json::json;

    use paddler_messaging::conversation_tool_call::ConversationToolCall;

    use super::ChatTemplateToolCall;

    #[test]
    fn a_tool_call_reaches_the_template_with_its_arguments_as_json_text() {
        assert_eq!(
            ChatTemplateToolCall(ConversationToolCall {
                arguments: json!({"location": "Paris"}).as_object().unwrap().clone(),
                id: "call_weather".to_owned(),
                name: "get_weather".to_owned(),
            })
            .into_chat_message_tool_call(),
            ChatMessageToolCall {
                id: "call_weather".to_owned(),
                name: "get_weather".to_owned(),
                arguments: r#"{"location":"Paris"}"#.to_owned(),
            }
        );
    }
}
