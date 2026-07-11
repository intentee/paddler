use serde::Deserialize;
use serde_json::Map;
use serde_json::Value;

use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;

#[derive(Deserialize)]
pub struct OpenAIToolParametersSchema {
    #[serde(default, rename = "type")]
    pub schema_type: Option<String>,
    #[serde(default)]
    pub properties: Option<Map<String, Value>>,
    #[serde(default)]
    pub required: Option<Vec<String>>,
    #[serde(default, rename = "additionalProperties")]
    pub additional_properties: Option<Value>,
}

impl OpenAIToolParametersSchema {
    #[must_use]
    pub fn into_raw_parameters_schema(self) -> RawParametersSchema {
        let Self {
            schema_type,
            properties,
            required,
            additional_properties,
        } = self;

        RawParametersSchema {
            schema_type: schema_type.unwrap_or_else(|| "object".to_owned()),
            properties,
            required,
            additional_properties,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::OpenAIToolParametersSchema;

    #[test]
    fn conversion_ignores_unknown_keywords_and_keeps_recognized_fields() {
        let schema: OpenAIToolParametersSchema = serde_json::from_value(json!({
            "type": "object",
            "properties": {"location": {"type": "string"}},
            "required": ["location"],
            "additionalProperties": false,
            "$schema": "http://json-schema.org/draft-07/schema#",
            "title": "Weather",
            "$defs": {"Unit": {"type": "string"}}
        }))
        .unwrap();

        let raw = schema.into_raw_parameters_schema();

        assert_eq!(raw.schema_type, "object");
        assert_eq!(raw.properties.unwrap().len(), 1);
        assert_eq!(raw.required, Some(vec!["location".to_owned()]));
        assert_eq!(raw.additional_properties, Some(json!(false)));
    }

    #[test]
    fn conversion_defaults_missing_type_to_object() {
        let schema: OpenAIToolParametersSchema = serde_json::from_value(json!({
            "properties": {"location": {"type": "string"}}
        }))
        .unwrap();

        let raw = schema.into_raw_parameters_schema();

        assert_eq!(raw.schema_type, "object");
        assert!(raw.required.is_none());
        assert!(raw.additional_properties.is_none());
    }
}
