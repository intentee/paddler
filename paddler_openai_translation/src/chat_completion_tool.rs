use serde::Deserialize;

use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;

use crate::function_definition::FunctionDefinition;

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum ChatCompletionTool {
    #[serde(rename = "function")]
    Function { function: Box<FunctionDefinition> },
}

impl ChatCompletionTool {
    #[must_use]
    pub fn into_tool(self) -> Tool<RawParametersSchema> {
        let Self::Function { function } = self;

        (*function).into_tool()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::from_value;
    use serde_json::json;

    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;

    use super::ChatCompletionTool;

    #[test]
    fn function_tool_converts_to_internal_tool() {
        let tool: ChatCompletionTool = from_value(json!({
            "type": "function",
            "function": {
                "name": "get_weather",
                "description": "fetch weather",
                "parameters": {"type": "object"}
            }
        }))
        .unwrap();

        let Tool::Function(function_call) = tool.into_tool();

        assert_eq!(function_call.function.name, "get_weather");
    }
}
