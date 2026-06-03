use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;

use crate::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;
use crate::validates::Validates;
use crate::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Parameters<TParametersSchema> {
    #[default]
    Empty,
    Schema(TParametersSchema),
}

impl<TParametersSchema> Parameters<TParametersSchema> {
    pub const fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }
}

impl Validates<Parameters<ValidatedParametersSchema>> for Parameters<RawParametersSchema> {
    fn validate(self) -> Result<Parameters<ValidatedParametersSchema>> {
        match self {
            Self::Empty => Ok(Parameters::Empty),
            Self::Schema(schema) => Ok(Parameters::Schema(schema.validate()?)),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Map;
    use serde_json::Value;

    use super::*;

    fn properties_with_name() -> Map<String, Value> {
        let mut properties = Map::new();
        properties.insert("name".to_owned(), Value::String("string".to_owned()));

        properties
    }

    #[test]
    fn is_empty_returns_true_for_empty_variant() {
        let parameters: Parameters<RawParametersSchema> = Parameters::Empty;

        assert!(parameters.is_empty());
    }

    #[test]
    fn is_empty_returns_false_for_schema_variant() {
        let parameters = Parameters::Schema(RawParametersSchema {
            schema_type: "object".to_owned(),
            properties: Some(Map::new()),
            required: None,
            additional_properties: None,
        });

        assert!(!parameters.is_empty());
    }

    #[test]
    fn validate_keeps_empty_variant_empty() {
        let parameters: Parameters<RawParametersSchema> = Parameters::Empty;

        let validated = parameters.validate().unwrap();

        assert!(validated.is_empty());
    }

    #[test]
    fn validate_carries_schema_into_validated_variant() {
        let parameters = Parameters::Schema(RawParametersSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties_with_name()),
            required: Some(vec!["name".to_owned()]),
            additional_properties: None,
        });

        let validated = parameters.validate().unwrap();

        assert!(!validated.is_empty());

        let expected = Parameters::Schema(ValidatedParametersSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties_with_name()),
            required: Some(vec!["name".to_owned()]),
            additional_properties: None,
        });

        assert_eq!(validated, expected);
    }
}
