use serde::Deserialize;

use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;

use crate::compatibility::openai_service::openai_chat_completion_function::OpenAIChatCompletionFunction;

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum OpenAIChatCompletionTool {
    #[serde(rename = "function")]
    Function {
        function: Box<OpenAIChatCompletionFunction>,
    },
    #[serde(other)]
    Unsupported,
}

impl OpenAIChatCompletionTool {
    #[must_use]
    pub fn into_tool(self) -> Option<Tool<RawParametersSchema>> {
        match self {
            Self::Function { function } => Some((*function).into_tool()),
            Self::Unsupported => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;

    use super::OpenAIChatCompletionTool;

    #[test]
    fn function_tool_converts_to_internal_tool() {
        let tool: OpenAIChatCompletionTool = serde_json::from_value(json!({
            "type": "function",
            "function": {
                "name": "get_weather",
                "description": "fetch weather",
                "parameters": {"type": "object"}
            }
        }))
        .unwrap();

        let Tool::Function(function_call) = tool.into_tool().unwrap();

        assert_eq!(function_call.function.name, "get_weather");
    }

    #[test]
    fn unsupported_tool_type_is_dropped() {
        let tool: OpenAIChatCompletionTool = serde_json::from_value(json!({
            "type": "web_search"
        }))
        .unwrap();

        assert!(tool.into_tool().is_none());
    }
}
