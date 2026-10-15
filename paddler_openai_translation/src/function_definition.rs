use serde::Deserialize;

use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::FunctionCall;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::function::Function;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters::Parameters;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;

use crate::tool_parameters_schema::ToolParametersSchema;

#[derive(Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub parameters: Option<ToolParametersSchema>,
}

impl FunctionDefinition {
    #[must_use]
    pub fn into_tool(self) -> Tool<RawParametersSchema> {
        let Self {
            name,
            description,
            parameters,
        } = self;

        Tool::Function(FunctionCall {
            function: Function {
                name,
                description: description.unwrap_or_default(),
                parameters: parameters.map_or(Parameters::Empty, |parameters| {
                    Parameters::Schema(parameters.into_raw_parameters_schema())
                }),
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::from_value;
    use serde_json::json;

    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;

    use super::FunctionDefinition;

    #[test]
    fn conversion_ignores_unknown_fields_and_builds_function_with_schema() {
        let function: FunctionDefinition = from_value(json!({
            "name": "get_weather",
            "description": "fetch weather",
            "parameters": {"type": "object", "properties": {"location": {"type": "string"}}},
            "strict": true
        }))
        .unwrap();

        let Tool::Function(function_call) = function.into_tool();

        assert_eq!(function_call.function.name, "get_weather");
        assert_eq!(function_call.function.description, "fetch weather");
        assert!(!function_call.function.parameters.is_empty());
    }

    #[test]
    fn conversion_defaults_missing_description_and_parameters() {
        let function: FunctionDefinition = from_value(json!({
            "name": "noop"
        }))
        .unwrap();

        let Tool::Function(function_call) = function.into_tool();

        assert_eq!(function_call.function.description, "");
        assert!(function_call.function.parameters.is_empty());
    }
}
