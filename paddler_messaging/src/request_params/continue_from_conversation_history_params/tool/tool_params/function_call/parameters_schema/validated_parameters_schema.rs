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

impl From<&ValidatedParametersSchema> for Value {
    fn from(
        ValidatedParametersSchema {
            schema_type,
            properties,
            required,
            additional_properties,
        }: &ValidatedParametersSchema,
    ) -> Self {
        let mut keywords = Map::new();

        keywords.insert("type".to_owned(), Self::String(schema_type.clone()));

        if let Some(properties) = properties {
            keywords.insert("properties".to_owned(), Self::Object(properties.clone()));
        }

        if let Some(required) = required {
            keywords.insert(
                "required".to_owned(),
                Self::Array(required.iter().cloned().map(Self::String).collect()),
            );
        }

        if let Some(additional_properties) = additional_properties {
            keywords.insert(
                "additionalProperties".to_owned(),
                additional_properties.clone(),
            );
        }

        Self::Object(keywords)
    }
}

impl Serialize for ValidatedParametersSchema {
    fn serialize<TSerializer>(
        &self,
        serializer: TSerializer,
    ) -> Result<TSerializer::Ok, TSerializer::Error>
    where
        TSerializer: Serializer,
    {
        Value::from(self).serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Map;
    use serde_json::Value;
    use serde_json::json;
    use serde_json::to_string;

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
            to_string(&schema).unwrap(),
            r#"{"type":"object","properties":{"location":{"type":"string"}},"required":["location"],"additionalProperties":false}"#
        );
    }

    #[test]
    fn omits_absent_keywords() {
        let schema = ValidatedParametersSchema {
            schema_type: "object".to_owned(),
            ..ValidatedParametersSchema::default()
        };

        assert_eq!(to_string(&schema).unwrap(), r#"{"type":"object"}"#);
    }
}
