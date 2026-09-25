use serde::Deserialize;
use serde::Serialize;
use serde::Serializer;
use serde_json::Map;
use serde_json::Value;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ValidatedParametersSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    pub properties: Option<Map<String, Value>>,
    pub required: Option<Vec<String>>,
    #[serde(rename = "additionalProperties")]
    pub additional_properties: Option<Value>,
}

impl ValidatedParametersSchema {
    #[must_use]
    pub fn to_json_schema(&self) -> Value {
        let mut document = Map::new();

        document.insert("type".to_owned(), Value::String(self.schema_type.clone()));

        if let Some(properties) = &self.properties {
            document.insert("properties".to_owned(), Value::Object(properties.clone()));
        }

        if let Some(required) = &self.required {
            document.insert(
                "required".to_owned(),
                Value::Array(required.iter().cloned().map(Value::String).collect()),
            );
        }

        if let Some(additional_properties) = &self.additional_properties {
            document.insert(
                "additionalProperties".to_owned(),
                additional_properties.clone(),
            );
        }

        Value::Object(document)
    }
}

impl Serialize for ValidatedParametersSchema {
    fn serialize<TSerializer: Serializer>(
        &self,
        serializer: TSerializer,
    ) -> Result<TSerializer::Ok, TSerializer::Error> {
        self.to_json_schema().serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Map;
    use serde_json::Value;
    use serde_json::json;

    use super::ValidatedParametersSchema;

    #[test]
    fn serializes_every_present_keyword_in_declaration_order() {
        let mut properties = Map::new();
        properties.insert("location".to_owned(), json!({"type": "string"}));

        let schema = ValidatedParametersSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["location".to_owned()]),
            additional_properties: Some(Value::Bool(false)),
        };

        assert_eq!(
            serde_json::to_string(&schema).unwrap(),
            r#"{"type":"object","properties":{"location":{"type":"string"}},"required":["location"],"additionalProperties":false}"#
        );
    }

    #[test]
    fn omits_absent_keywords() {
        let schema = ValidatedParametersSchema {
            schema_type: "object".to_owned(),
            ..ValidatedParametersSchema::default()
        };

        assert_eq!(
            serde_json::to_string(&schema).unwrap(),
            r#"{"type":"object"}"#
        );
    }
}
