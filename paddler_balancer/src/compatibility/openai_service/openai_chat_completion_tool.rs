use serde::Deserialize;

use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;

use crate::compatibility::openai_service::openai_function_definition::OpenAIFunctionDefinition;

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum OpenAIChatCompletionTool {
    #[serde(rename = "function")]
    Function {
        function: Box<OpenAIFunctionDefinition>,
    },
}

impl OpenAIChatCompletionTool {
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

    use super::OpenAIChatCompletionTool;

    #[test]
    fn function_tool_converts_to_internal_tool() {
        let tool: OpenAIChatCompletionTool = from_value(json!({
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
