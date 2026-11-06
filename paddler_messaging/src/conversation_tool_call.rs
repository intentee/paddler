use llama_cpp_bindings_types::chat_message_tool_call::ChatMessageToolCall;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationToolCall {
    pub arguments: Map<String, Value>,
    pub id: String,
    pub name: String,
}

impl From<ConversationToolCall> for ChatMessageToolCall {
    fn from(
        ConversationToolCall {
            arguments,
            id,
            name,
        }: ConversationToolCall,
    ) -> Self {
        Self {
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

    use super::ConversationToolCall;

    #[test]
    fn a_tool_call_reaches_the_template_with_its_arguments_as_json_text() {
        assert_eq!(
            ChatMessageToolCall::from(ConversationToolCall {
                arguments: json!({"location": "Paris"}).as_object().unwrap().clone(),
                id: "call_weather".to_owned(),
                name: "get_weather".to_owned(),
            }),
            ChatMessageToolCall {
                id: "call_weather".to_owned(),
                name: "get_weather".to_owned(),
                arguments: r#"{"location":"Paris"}"#.to_owned(),
            }
        );
    }
}
