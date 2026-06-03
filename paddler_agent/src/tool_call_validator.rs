use std::collections::HashMap;

use jsonschema::Validator;
use llama_cpp_bindings::ParsedToolCall;
use llama_cpp_bindings::ToolCallArguments;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters::Parameters;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;

use paddler_messaging::tool_call_validation_error::ToolCallValidationError;

use crate::validator_build_error::ValidatorBuildError;

enum ValidationStrategy {
    JsonObjectOnly,
    Schema(Box<Validator>),
}

pub struct ToolCallValidator {
    strategies: HashMap<String, ValidationStrategy>,
}

impl ToolCallValidator {
    pub fn from_tools(
        tools: &[Tool<ValidatedParametersSchema>],
    ) -> Result<Self, ValidatorBuildError> {
        let mut strategies = HashMap::with_capacity(tools.len());

        for tool in tools {
            let Tool::Function(function_call) = tool;
            let function = &function_call.function;

            let strategy = match &function.parameters {
                Parameters::Empty => ValidationStrategy::JsonObjectOnly,
                Parameters::Schema(schema) => {
                    let schema_value = serde_json::to_value(schema).map_err(|err| {
                        ValidatorBuildError::SerializationFailed {
                            tool_name: function.name.clone(),
                            message: err.to_string(),
                        }
                    })?;
                    let compiled = jsonschema::validator_for(&schema_value).map_err(|err| {
                        ValidatorBuildError::InvalidSchema {
                            tool_name: function.name.clone(),
                            message: err.to_string(),
                        }
                    })?;
                    ValidationStrategy::Schema(Box::new(compiled))
                }
            };

            strategies.insert(function.name.clone(), strategy);
        }

        Ok(Self { strategies })
    }

    pub fn validate(&self, parsed: &ParsedToolCall) -> Result<(), ToolCallValidationError> {
        let strategy = self
            .strategies
            .get(&parsed.name)
            .ok_or_else(|| ToolCallValidationError::UnknownToolName(parsed.name.clone()))?;

        let arguments_value = match &parsed.arguments {
            ToolCallArguments::ValidJson(value) => value,
            ToolCallArguments::InvalidJson(_) => return Ok(()),
        };

        match strategy {
            ValidationStrategy::JsonObjectOnly => Ok(()),
            ValidationStrategy::Schema(validator) => {
                let mut messages: Vec<String> = validator
                    .iter_errors(arguments_value)
                    .map(|err| err.to_string())
                    .collect();

                if messages.is_empty() {
                    Ok(())
                } else {
                    Err(ToolCallValidationError::SchemaMismatch {
                        tool_name: parsed.name.clone(),
                        message: messages.remove(0),
                    })
                }
            }
        }
    }

    #[must_use]
    pub fn known_tool_names(&self) -> Vec<&str> {
        self.strategies.keys().map(String::as_str).collect()
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings::ParsedToolCall;
    use llama_cpp_bindings::ToolCallArguments;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::FunctionCall;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::function::Function;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters::Parameters;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
    use serde_json::Map;
    use serde_json::Value;
    use serde_json::json;

    use super::ToolCallValidator;
    use crate::validator_build_error::ValidatorBuildError;
    use paddler_messaging::tool_call_validation_error::ToolCallValidationError;

    fn valid_json_arguments(value: Value) -> ToolCallArguments {
        ToolCallArguments::ValidJson(value)
    }

    fn weather_tool_with_schema() -> Tool<ValidatedParametersSchema> {
        let mut properties = Map::new();
        properties.insert(
            "location".to_owned(),
            serde_json::json!({"type": "string", "description": "city"}),
        );

        Tool::Function(FunctionCall {
            function: Function {
                name: "get_weather".to_owned(),
                description: "fetch weather".to_owned(),
                parameters: Parameters::Schema(ValidatedParametersSchema {
                    schema_type: "object".to_owned(),
                    properties: Some(properties),
                    required: Some(vec!["location".to_owned()]),
                    additional_properties: Some(Value::Bool(false)),
                }),
            },
        })
    }

    fn schemaless_tool() -> Tool<ValidatedParametersSchema> {
        Tool::Function(FunctionCall {
            function: Function {
                name: "freeform".to_owned(),
                description: "tool with no schema".to_owned(),
                parameters: Parameters::Empty,
            },
        })
    }

    #[test]
    fn schema_validator_accepts_matching_arguments() {
        let validator = ToolCallValidator::from_tools(&[weather_tool_with_schema()]).unwrap();
        let parsed = ParsedToolCall::new(
            "id".to_owned(),
            "get_weather".to_owned(),
            valid_json_arguments(json!({"location": "Paris"})),
        );

        assert!(validator.validate(&parsed).is_ok());
    }

    #[test]
    fn schema_validator_rejects_missing_required_field() {
        let validator = ToolCallValidator::from_tools(&[weather_tool_with_schema()]).unwrap();
        let parsed = ParsedToolCall::new(
            "id".to_owned(),
            "get_weather".to_owned(),
            valid_json_arguments(json!({})),
        );

        let validation_error = validator.validate(&parsed).err().unwrap();

        assert!(matches!(
            validation_error,
            ToolCallValidationError::SchemaMismatch { tool_name, .. } if tool_name == "get_weather"
        ));
    }

    #[test]
    fn schema_validator_rejects_wrong_type() {
        let validator = ToolCallValidator::from_tools(&[weather_tool_with_schema()]).unwrap();
        let parsed = ParsedToolCall::new(
            "id".to_owned(),
            "get_weather".to_owned(),
            valid_json_arguments(json!({"location": 42})),
        );

        let validation_error = validator.validate(&parsed).err().unwrap();

        assert!(matches!(
            validation_error,
            ToolCallValidationError::SchemaMismatch { tool_name, .. } if tool_name == "get_weather"
        ));
    }

    #[test]
    fn unknown_tool_name_returns_error() {
        let validator = ToolCallValidator::from_tools(&[weather_tool_with_schema()]).unwrap();
        let parsed = ParsedToolCall::new(
            "id".to_owned(),
            "set_thermostat".to_owned(),
            valid_json_arguments(json!({"value": 21})),
        );

        let validation_error = validator.validate(&parsed).err().unwrap();

        assert!(matches!(
            validation_error,
            ToolCallValidationError::UnknownToolName(name) if name == "set_thermostat"
        ));
    }

    #[test]
    fn invalid_json_arguments_pass_validation_silently() {
        let validator = ToolCallValidator::from_tools(&[weather_tool_with_schema()]).unwrap();
        let parsed = ParsedToolCall::new(
            "id".to_owned(),
            "get_weather".to_owned(),
            ToolCallArguments::InvalidJson("not json".to_owned()),
        );

        assert!(validator.validate(&parsed).is_ok());
    }

    #[test]
    fn schemaless_tool_accepts_any_object() {
        let validator = ToolCallValidator::from_tools(&[schemaless_tool()]).unwrap();
        let parsed = ParsedToolCall::new(
            "id".to_owned(),
            "freeform".to_owned(),
            valid_json_arguments(json!({"x": 1, "y": 2})),
        );

        assert!(validator.validate(&parsed).is_ok());
    }

    #[test]
    fn known_tool_names_returns_all_registered_names() {
        let validator =
            ToolCallValidator::from_tools(&[weather_tool_with_schema(), schemaless_tool()])
                .unwrap();

        let mut names = validator.known_tool_names();
        names.sort_unstable();

        assert_eq!(names, vec!["freeform", "get_weather"]);
    }

    #[test]
    fn empty_tools_yields_validator_that_rejects_any_call() {
        let validator = ToolCallValidator::from_tools(&[]).unwrap();
        let parsed = ParsedToolCall::new(
            "id".to_owned(),
            "anything".to_owned(),
            valid_json_arguments(json!({})),
        );

        let validation_error = validator.validate(&parsed).err().unwrap();

        assert!(matches!(
            validation_error,
            ToolCallValidationError::UnknownToolName(name) if name == "anything"
        ));
    }

    fn tool_with_invalid_property_schema() -> Tool<ValidatedParametersSchema> {
        let mut properties = Map::new();
        properties.insert("location".to_owned(), serde_json::json!({"type": 42}));

        Tool::Function(FunctionCall {
            function: Function {
                name: "broken_tool".to_owned(),
                description: "tool whose property schema is not valid JSON Schema".to_owned(),
                parameters: Parameters::Schema(ValidatedParametersSchema {
                    schema_type: "object".to_owned(),
                    properties: Some(properties),
                    required: None,
                    additional_properties: None,
                }),
            },
        })
    }

    #[test]
    fn invalid_property_schema_rejects_validator_build() {
        let build_error = ToolCallValidator::from_tools(&[tool_with_invalid_property_schema()])
            .err()
            .unwrap();

        assert!(matches!(
            build_error,
            ValidatorBuildError::InvalidSchema { tool_name, .. } if tool_name == "broken_tool"
        ));
    }

    fn tool_with_invalid_additional_properties_schema() -> Tool<ValidatedParametersSchema> {
        Tool::Function(FunctionCall {
            function: Function {
                name: "broken_additional".to_owned(),
                description: "tool whose additionalProperties schema is invalid".to_owned(),
                parameters: Parameters::Schema(ValidatedParametersSchema {
                    schema_type: "object".to_owned(),
                    properties: None,
                    required: None,
                    additional_properties: Some(json!({"type": "not_a_type"})),
                }),
            },
        })
    }

    #[test]
    fn invalid_additional_properties_schema_rejects_validator_build() {
        let build_error =
            ToolCallValidator::from_tools(&[tool_with_invalid_additional_properties_schema()])
                .err()
                .unwrap();

        assert!(matches!(
            build_error,
            ValidatorBuildError::InvalidSchema { tool_name, .. } if tool_name == "broken_additional"
        ));
    }
}
