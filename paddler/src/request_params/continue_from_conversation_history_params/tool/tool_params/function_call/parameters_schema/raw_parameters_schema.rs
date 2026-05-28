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
    use serde_json::from_value;
    use serde_json::json;

    use super::*;

    #[test]
    fn test_deserialize_with_valid_properties() -> Result<()> {
        let input = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer", "minimum": 0}
            },
            "required": ["name"],
            "additionalProperties": false
        });

        let raw_schema: RawParametersSchema = from_value(input)?;
        let schema: ValidatedParametersSchema = raw_schema.validate()?;

        assert_eq!(schema.schema_type, "object");
        assert!(schema.properties.is_some());

        let properties = schema
            .properties
            .as_ref()
            .ok_or_else(|| anyhow!("expected properties"))?;

        assert_eq!(properties.len(), 2);
        assert_eq!(schema.required, Some(vec!["name".to_owned()]));
        assert_eq!(schema.additional_properties, Some(json!(false)));

        Ok(())
    }

    #[test]
    fn test_deserialize_required_field_not_in_properties() -> Result<()> {
        let input = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            },
            "required": ["name", "missing_field"]
        });

        let raw_schema: RawParametersSchema = from_value(input)?;
        let result: Result<ValidatedParametersSchema, _> = raw_schema.validate();

        assert!(result.is_err());

        if let Err(error) = &result {
            assert!(
                error
                    .to_string()
                    .contains("Required field 'missing_field' not found in properties")
            );
        }

        Ok(())
    }
}
