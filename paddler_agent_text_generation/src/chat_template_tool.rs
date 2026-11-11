use llama_cpp_bindings_types::chat_tool::ChatTool;
use serde_json::Value;

use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::FunctionCall;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::function::Function;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters::Parameters;

pub struct ChatTemplateTool<'tool>(pub &'tool Tool<Value>);

impl ChatTemplateTool<'_> {
    #[must_use]
    pub fn to_chat_tool(&self) -> ChatTool {
        let Tool::Function(FunctionCall {
            function:
                Function {
                    name,
                    description,
                    parameters,
                },
        }) = self.0;

        ChatTool {
            name: name.clone(),
            description: description.clone(),
            parameters: match parameters {
                Parameters::Schema(schema) => Some(schema.clone()),
                Parameters::Empty => None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings_types::chat_tool::ChatTool;
    use serde_json::Value;
    use serde_json::json;

    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::FunctionCall;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::function::Function;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters::Parameters;

    use super::ChatTemplateTool;

    fn weather_tool(parameters: Parameters<Value>) -> Tool<Value> {
        Tool::Function(FunctionCall {
            function: Function {
                name: "get_weather".to_owned(),
                description: "Reports the weather".to_owned(),
                parameters,
            },
        })
    }

    #[test]
    fn a_tool_with_parameters_reaches_the_template_with_its_schema() {
        let schema = json!({"type": "object", "properties": {"location": {"type": "string"}}});

        assert_eq!(
            ChatTemplateTool(&weather_tool(Parameters::Schema(schema.clone()))).to_chat_tool(),
            ChatTool {
                name: "get_weather".to_owned(),
                description: "Reports the weather".to_owned(),
                parameters: Some(schema),
            }
        );
    }

    #[test]
    fn a_tool_without_parameters_reaches_the_template_without_them() {
        assert_eq!(
            ChatTemplateTool(&weather_tool(Parameters::Empty)).to_chat_tool(),
            ChatTool {
                name: "get_weather".to_owned(),
                description: "Reports the weather".to_owned(),
                parameters: None,
            }
        );
    }
}
