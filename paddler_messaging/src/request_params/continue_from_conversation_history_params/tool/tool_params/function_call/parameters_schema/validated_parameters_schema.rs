use serde::Deserialize;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ValidatedParametersSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<Map<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    #[serde(
        rename = "additionalProperties",
        skip_serializing_if = "Option::is_none"
    )]
    pub additional_properties: Option<Value>,
}

impl From<ValidatedParametersSchema> for Value {
    fn from(
        ValidatedParametersSchema {
            schema_type,
            properties,
            required,
            additional_properties,
        }: ValidatedParametersSchema,
    ) -> Self {
        let mut keywords = Map::new();

        keywords.insert("type".to_owned(), Self::String(schema_type));

        if let Some(properties) = properties {
            keywords.insert("properties".to_owned(), Self::Object(properties));
        }

        if let Some(required) = required {
            keywords.insert(
                "required".to_owned(),
                Self::Array(required.into_iter().map(Self::String).collect()),
            );
        }

        if let Some(additional_properties) = additional_properties {
            keywords.insert("additionalProperties".to_owned(), additional_properties);
        }

        Self::Object(keywords)
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
