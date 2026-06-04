use anyhow::Result;
use anyhow::anyhow;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;

use super::validated_parameters_schema::ValidatedParametersSchema;
use crate::validates::Validates;

#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawParametersSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    pub properties: Option<Map<String, Value>>,
    pub required: Option<Vec<String>>,
    #[serde(rename = "additionalProperties")]
    pub additional_properties: Option<Value>,
}

impl Validates<ValidatedParametersSchema> for RawParametersSchema {
    fn validate(self) -> Result<ValidatedParametersSchema> {
        if let (Some(required), Some(properties)) = (&self.required, &self.properties) {
            for field in required {
                if !properties.contains_key(field) {
                    return Err(anyhow!("Required field '{field}' not found in properties"));
                }
            }
        }

        Ok(ValidatedParametersSchema {
            schema_type: self.schema_type,
            properties: self.properties,
            required: self.required,
            additional_properties: self.additional_properties,
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn validate_passes_when_every_required_field_is_present() {
        let mut properties = Map::new();
        properties.insert("name".to_owned(), json!({"type": "string"}));
        properties.insert("age".to_owned(), json!({"type": "integer"}));

        let raw_schema = RawParametersSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["name".to_owned()]),
            additional_properties: Some(json!(false)),
        };

        let schema = raw_schema.validate().unwrap();

        assert_eq!(schema.schema_type, "object");
        assert_eq!(schema.properties.as_ref().unwrap().len(), 2);
        assert_eq!(schema.required, Some(vec!["name".to_owned()]));
        assert_eq!(schema.additional_properties, Some(json!(false)));
    }

    #[test]
    fn validate_passes_when_required_is_absent() {
        let mut properties = Map::new();
        properties.insert("name".to_owned(), json!({"type": "string"}));

        let raw_schema = RawParametersSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: None,
            additional_properties: None,
        };

        let schema = raw_schema.validate().unwrap();

        assert_eq!(schema.schema_type, "object");
        assert_eq!(schema.required, None);
        assert_eq!(schema.additional_properties, None);
    }

    #[test]
    fn validate_fails_when_required_field_is_missing_from_properties() {
        let mut properties = Map::new();
        properties.insert("name".to_owned(), json!({"type": "string"}));

        let raw_schema = RawParametersSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["name".to_owned(), "missing_field".to_owned()]),
            additional_properties: None,
        };

        let error = raw_schema.validate().unwrap_err();

        assert_eq!(
            error.to_string(),
            "Required field 'missing_field' not found in properties"
        );
    }
}
