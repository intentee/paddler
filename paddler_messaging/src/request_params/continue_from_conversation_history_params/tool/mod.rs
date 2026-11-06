pub mod tool_params;

use llama_cpp_bindings_types::chat_tool::ChatTool;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;
use crate::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use crate::request_params_validation_error::RequestParamsValidationError;
use crate::validates::Validates;
use self::tool_params::function_call::FunctionCall;
use self::tool_params::function_call::function::Function;
use self::tool_params::function_call::parameters::Parameters;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(tag = "type")]
pub enum Tool<TParametersSchema> {
    #[serde(rename = "function")]
    Function(FunctionCall<TParametersSchema>),
}

impl Validates<Tool<ValidatedParametersSchema>> for Tool<RawParametersSchema> {
    fn validate(self) -> Result<Tool<ValidatedParametersSchema>, RequestParamsValidationError> {
        match self {
            Self::Function(function_call) => Ok(Tool::Function(function_call.validate()?)),
        }
    }
}

impl From<Tool<ValidatedParametersSchema>> for Tool<Value> {
    fn from(tool: Tool<ValidatedParametersSchema>) -> Self {
        match tool {
            Tool::Function(function_call) => Self::Function(FunctionCall::from(function_call)),
        }
    }
}

impl From<&Tool<Value>> for ChatTool {
    fn from(tool: &Tool<Value>) -> Self {
        let Tool::Function(FunctionCall {
            function:
                Function {
                    name,
                    description,
                    parameters,
                },
        }) = tool;

        Self {
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

    use super::Tool;
    use super::tool_params::function_call::FunctionCall;
    use super::tool_params::function_call::function::Function;
    use super::tool_params::function_call::parameters::Parameters;

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
            ChatTool::from(&weather_tool(Parameters::Schema(schema.clone()))),
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
            ChatTool::from(&weather_tool(Parameters::Empty)),
            ChatTool {
                name: "get_weather".to_owned(),
                description: "Reports the weather".to_owned(),
                parameters: None,
            }
        );
    }
}
