use std::collections::HashMap;

use jsonschema::Validator;
use jsonschema::validator_for;
use llama_cpp_bindings::ParsedToolCall;
use llama_cpp_bindings::ToolCallArguments;
use paddler_messaging::tool_call_validation_error::ToolCallValidationError;
use serde_json::Value;

use crate::validator_build_error::ValidatorBuildError;

enum ValidationStrategy {
    JsonObjectOnly,
    Schema(Box<Validator>),
}

pub struct ToolCallValidator {
    strategies: HashMap<String, ValidationStrategy>,
}

impl ToolCallValidator {
    pub fn from_tools(tools: &[Value]) -> Result<Self, ValidatorBuildError> {
        let mut strategies = HashMap::with_capacity(tools.len());

        for (tool_index, tool) in tools.iter().enumerate() {
            let function = tool.get("function").and_then(Value::as_object).ok_or(
                ValidatorBuildError::InvalidSerializedTool {
                    tool_index,
                    message: "function must be an object",
                },
            )?;
            let name = function.get("name").and_then(Value::as_str).ok_or(
                ValidatorBuildError::InvalidSerializedTool {
                    tool_index,
                    message: "function.name must be a string",
                },
            )?;

            let strategy = match function.get("parameters") {
                None => ValidationStrategy::JsonObjectOnly,
                Some(schema) => {
                    let compiled = validator_for(schema).map_err(|err| {
                        ValidatorBuildError::InvalidSchema {
                            tool_name: name.to_owned(),
                            message: err.to_string(),
                        }
                    })?;
                    ValidationStrategy::Schema(Box::new(compiled))
                }
            };

            strategies.insert(name.to_owned(), strategy);
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
    use serde_json::Value;
    use serde_json::json;

    use super::ToolCallValidator;
    use crate::validator_build_error::ValidatorBuildError;
    use paddler_messaging::tool_call_validation_error::ToolCallValidationError;

    fn valid_json_arguments(value: Value) -> ToolCallArguments {
        ToolCallArguments::ValidJson(value)
    }

    fn weather_tool_with_schema() -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "get_weather",
                "description": "fetch weather",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "location": {"type": "string", "description": "city"}
                    },
                    "required": ["location"],
                    "additionalProperties": false
                }
            }
        })
    }

    fn schemaless_tool() -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "freeform",
                "description": "tool with no schema"
            }
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

    fn tool_with_invalid_property_schema() -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "broken_tool",
                "description": "tool whose property schema is not valid JSON Schema",
                "parameters": {
                    "type": "object",
                    "properties": {"location": {"type": 42}}
                }
            }
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

    fn tool_with_invalid_additional_properties_schema() -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "broken_additional",
                "description": "tool whose additionalProperties schema is invalid",
                "parameters": {
                    "type": "object",
                    "additionalProperties": {"type": "not_a_type"}
                }
            }
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

    #[test]
    fn missing_function_object_rejects_validator_build() {
        let build_error = ToolCallValidator::from_tools(&[json!({"type": "function"})])
            .err()
            .unwrap();

        assert!(matches!(
            build_error,
            ValidatorBuildError::InvalidSerializedTool {
                tool_index: 0,
                message: "function must be an object"
            }
        ));
    }

    #[test]
    fn non_string_function_name_rejects_validator_build() {
        let build_error = ToolCallValidator::from_tools(&[json!({
            "type": "function",
            "function": {"name": 42}
        })])
        .err()
        .unwrap();

        assert!(matches!(
            build_error,
            ValidatorBuildError::InvalidSerializedTool {
                tool_index: 0,
                message: "function.name must be a string"
            }
        ));
    }
}
