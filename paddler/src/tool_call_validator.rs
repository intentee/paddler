use std::collections::HashMap;

use jsonschema::Validator;
use paddler_types::parsed_tool_call::ParsedToolCall;
use paddler_types::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters::Parameters;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use serde_json::Value;

use crate::tool_call_validation_error::ToolCallValidationError;

/// Why building the validator failed. Either we couldn't render the supplied
/// parameters schema as JSON or the JSON we got back wasn't a valid
/// JSON-Schema document.
#[derive(Debug, thiserror::Error)]
pub enum ValidatorBuildError {
    #[error("could not serialize tool {tool_name:?} parameters to JSON: {message}")]
    SerializationFailed { tool_name: String, message: String },
    #[error("tool {tool_name:?} parameters are not a valid JSON Schema: {message}")]
    InvalidSchema { tool_name: String, message: String },
}

/// Per-tool validation strategy.
///
/// `JsonObjectOnly` is the fallback when the request didn't supply a JSON
/// schema for the tool's parameters — we still want to confirm the parser
/// returned a JSON object and not a stray scalar/array. `Schema` runs the
/// full `jsonschema::Validator`.
enum ValidationStrategy {
    JsonObjectOnly,
    Schema(Box<Validator>),
}

/// Validates [`ParsedToolCall`] payloads coming out of the bindings parser.
///
/// Single responsibility: given a parsed tool call, decide whether the
/// arguments are well-formed under the request's tool definitions. The
/// validator is **always** consulted by the pipeline — when no schema was
/// declared for a tool the strategy defaults to a JSON-object structural
/// check rather than skipping validation entirely.
pub struct ToolCallValidator {
    strategies: HashMap<String, ValidationStrategy>,
}

impl ToolCallValidator {
    /// Build a validator from the request's tools array. Tools with
    /// `Parameters::Empty` get the `JsonObjectOnly` fallback; tools with
    /// `Parameters::Schema(...)` get a compiled `jsonschema::Validator`.
    ///
    /// # Errors
    /// Returns the underlying `jsonschema` error when a declared schema is
    /// itself invalid — this is a request-time validation failure.
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

    /// Validate the parsed tool call against its tool's strategy. Returns
    /// `Ok(())` on success and a specific [`ToolCallValidationError`]
    /// variant on failure.
    pub fn validate(&self, parsed: &ParsedToolCall) -> Result<(), ToolCallValidationError> {
        let strategy = self.strategies.get(&parsed.name).ok_or_else(|| {
            ToolCallValidationError::UnknownToolName(parsed.name.clone())
        })?;

        let arguments_value: Value = serde_json::from_str(&parsed.arguments_json).map_err(|err| {
            ToolCallValidationError::InvalidJson {
                tool_name: parsed.name.clone(),
                message: err.to_string(),
            }
        })?;

        if !arguments_value.is_object() {
            return Err(ToolCallValidationError::NotAnObject {
                tool_name: parsed.name.clone(),
                kind: json_value_kind(&arguments_value),
            });
        }

        match strategy {
            ValidationStrategy::JsonObjectOnly => Ok(()),
            ValidationStrategy::Schema(validator) => {
                let mut messages: Vec<String> =
                    validator.iter_errors(&arguments_value).map(|err| err.to_string()).collect();

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

const fn json_value_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use paddler_types::parsed_tool_call::ParsedToolCall;
    use paddler_types::request_params::continue_from_conversation_history_params::tool::Tool;
    use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::FunctionCall;
    use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::function::Function;
    use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters::Parameters;
    use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
    use serde_json::Map;
    use serde_json::Value;

    use super::ToolCallValidator;
    use super::json_value_kind;
    use crate::tool_call_validation_error::ToolCallValidationError;

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
        let validator =
            ToolCallValidator::from_tools(&[weather_tool_with_schema()]).unwrap();
        let parsed = ParsedToolCall::new(
            "id".to_owned(),
            "get_weather".to_owned(),
            "{\"location\":\"Paris\"}".to_owned(),
        );

        assert!(validator.validate(&parsed).is_ok());
    }

    #[test]
    fn schema_validator_rejects_missing_required_field() {
        let validator =
            ToolCallValidator::from_tools(&[weather_tool_with_schema()]).unwrap();
        let parsed = ParsedToolCall::new(
            "id".to_owned(),
            "get_weather".to_owned(),
            "{}".to_owned(),
        );

        match validator.validate(&parsed) {
            Err(ToolCallValidationError::SchemaMismatch { tool_name, .. }) => {
                assert_eq!(tool_name, "get_weather");
            }
            other => panic!("expected SchemaMismatch, got {other:?}"),
        }
    }

    #[test]
    fn schema_validator_rejects_wrong_type() {
        let validator =
            ToolCallValidator::from_tools(&[weather_tool_with_schema()]).unwrap();
        let parsed = ParsedToolCall::new(
            "id".to_owned(),
            "get_weather".to_owned(),
            "{\"location\":42}".to_owned(),
        );

        match validator.validate(&parsed) {
            Err(ToolCallValidationError::SchemaMismatch { .. }) => {}
            other => panic!("expected SchemaMismatch, got {other:?}"),
        }
    }

    #[test]
    fn unknown_tool_name_returns_error() {
        let validator =
            ToolCallValidator::from_tools(&[weather_tool_with_schema()]).unwrap();
        let parsed = ParsedToolCall::new(
            "id".to_owned(),
            "set_thermostat".to_owned(),
            "{\"value\":21}".to_owned(),
        );

        match validator.validate(&parsed) {
            Err(ToolCallValidationError::UnknownToolName(name)) => {
                assert_eq!(name, "set_thermostat");
            }
            other => panic!("expected UnknownToolName, got {other:?}"),
        }
    }

    #[test]
    fn invalid_json_returns_invalid_json_error() {
        let validator =
            ToolCallValidator::from_tools(&[weather_tool_with_schema()]).unwrap();
        let parsed = ParsedToolCall::new(
            "id".to_owned(),
            "get_weather".to_owned(),
            "not json".to_owned(),
        );

        match validator.validate(&parsed) {
            Err(ToolCallValidationError::InvalidJson { tool_name, .. }) => {
                assert_eq!(tool_name, "get_weather");
            }
            other => panic!("expected InvalidJson, got {other:?}"),
        }
    }

    #[test]
    fn non_object_arguments_return_not_an_object_error() {
        let validator = ToolCallValidator::from_tools(&[schemaless_tool()]).unwrap();
        let parsed = ParsedToolCall::new(
            "id".to_owned(),
            "freeform".to_owned(),
            "[1, 2, 3]".to_owned(),
        );

        match validator.validate(&parsed) {
            Err(ToolCallValidationError::NotAnObject { tool_name, kind }) => {
                assert_eq!(tool_name, "freeform");
                assert_eq!(kind, "array");
            }
            other => panic!("expected NotAnObject, got {other:?}"),
        }
    }

    #[test]
    fn schemaless_tool_accepts_any_object() {
        let validator = ToolCallValidator::from_tools(&[schemaless_tool()]).unwrap();
        let parsed = ParsedToolCall::new(
            "id".to_owned(),
            "freeform".to_owned(),
            "{\"x\":1,\"y\":2}".to_owned(),
        );

        assert!(validator.validate(&parsed).is_ok());
    }

    #[test]
    fn known_tool_names_returns_all_registered_names() {
        let validator = ToolCallValidator::from_tools(&[
            weather_tool_with_schema(),
            schemaless_tool(),
        ])
        .unwrap();

        let mut names = validator.known_tool_names();
        names.sort_unstable();

        assert_eq!(names, vec!["freeform", "get_weather"]);
    }

    #[test]
    fn json_value_kind_reports_each_kind() {
        assert_eq!(json_value_kind(&Value::Null), "null");
        assert_eq!(json_value_kind(&Value::Bool(true)), "bool");
        assert_eq!(json_value_kind(&Value::Number(serde_json::Number::from(1))), "number");
        assert_eq!(json_value_kind(&Value::String("x".to_owned())), "string");
        assert_eq!(json_value_kind(&Value::Array(vec![])), "array");
        assert_eq!(json_value_kind(&Value::Object(Map::new())), "object");
    }

    #[test]
    fn empty_tools_yields_validator_that_rejects_any_call() {
        let validator = ToolCallValidator::from_tools(&[]).unwrap();
        let parsed = ParsedToolCall::new(
            "id".to_owned(),
            "anything".to_owned(),
            "{}".to_owned(),
        );

        assert!(matches!(
            validator.validate(&parsed),
            Err(ToolCallValidationError::UnknownToolName(_))
        ));
    }
}
