use std::fmt;
use std::fmt::Formatter;

use serde::Deserialize;
use serde::Deserializer;
use serde::de::Error as _;
use serde::de::IgnoredAny;
use serde::de::MapAccess;
use serde::de::SeqAccess;
use serde::de::Visitor;
use serde_json::value::RawValue;

use crate::responses_text_format_field::ResponsesTextFormatField;
use crate::responses_text_format_refusal::ResponsesTextFormatRefusal;
use crate::responses_text_format_schema_occurrences::ResponsesTextFormatSchemaOccurrences;
use crate::responses_text_format_type::ResponsesTextFormatType;

fn count_leftover_elements<'de, TSeqAccess>(
    mut format_elements: TSeqAccess,
) -> Result<usize, TSeqAccess::Error>
where
    TSeqAccess: SeqAccess<'de>,
{
    let mut leftover_elements = 0;

    while format_elements.next_element::<IgnoredAny>()?.is_some() {
        leftover_elements += 1;
    }

    Ok(leftover_elements)
}

fn without_leftover_elements(
    format: ResponsesTextFormat,
    consumed_elements: usize,
    leftover_elements: usize,
) -> Result<ResponsesTextFormat, ResponsesTextFormatRefusal> {
    if leftover_elements == 0 {
        Ok(format)
    } else {
        Err(ResponsesTextFormatRefusal::ElementsLeftOver {
            consumed_elements,
            leftover_elements,
        })
    }
}

struct ResponsesTextFormatVisitor;

impl<'de> Visitor<'de> for ResponsesTextFormatVisitor {
    type Value = Result<ResponsesTextFormat, ResponsesTextFormatRefusal>;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("internally tagged enum ResponsesTextFormat")
    }

    fn visit_seq<TSeqAccess>(
        self,
        mut format_elements: TSeqAccess,
    ) -> Result<Self::Value, TSeqAccess::Error>
    where
        TSeqAccess: SeqAccess<'de>,
    {
        match format_elements.next_element::<ResponsesTextFormatType>()? {
            None => Err(TSeqAccess::Error::missing_field("type")),
            Some(ResponsesTextFormatType::Text) => Ok(without_leftover_elements(
                ResponsesTextFormat::Text,
                0,
                count_leftover_elements(format_elements)?,
            )),
            Some(ResponsesTextFormatType::JsonSchema) => {
                match format_elements.next_element::<Box<RawValue>>()? {
                    None => Ok(Err(ResponsesTextFormatRefusal::SchemaElementMissing)),
                    Some(schema) => Ok(without_leftover_elements(
                        ResponsesTextFormat::JsonSchema { schema },
                        1,
                        count_leftover_elements(format_elements)?,
                    )),
                }
            }
        }
    }

    fn visit_map<TMapAccess>(
        self,
        mut format_fields: TMapAccess,
    ) -> Result<Self::Value, TMapAccess::Error>
    where
        TMapAccess: MapAccess<'de>,
    {
        let mut format_type = None;
        let mut schema_occurrences = ResponsesTextFormatSchemaOccurrences::Absent;

        while let Some(format_field) = format_fields.next_key()? {
            match format_field {
                ResponsesTextFormatField::Schema => {
                    schema_occurrences = schema_occurrences
                        .with_another(format_fields.next_value::<Box<RawValue>>()?);
                }
                ResponsesTextFormatField::Type => {
                    if format_type.is_some() {
                        return Err(TMapAccess::Error::duplicate_field("type"));
                    }

                    format_type = Some(format_fields.next_value::<ResponsesTextFormatType>()?);
                }
                ResponsesTextFormatField::Unrecognized => {
                    format_fields.next_value::<IgnoredAny>()?;
                }
            }
        }

        match format_type {
            None => Err(TMapAccess::Error::missing_field("type")),
            Some(ResponsesTextFormatType::Text) => Ok(Ok(ResponsesTextFormat::Text)),
            Some(ResponsesTextFormatType::JsonSchema) => Ok(match schema_occurrences {
                ResponsesTextFormatSchemaOccurrences::Absent => {
                    Err(ResponsesTextFormatRefusal::SchemaMissing)
                }
                ResponsesTextFormatSchemaOccurrences::Once(schema) => {
                    Ok(ResponsesTextFormat::JsonSchema { schema })
                }
                ResponsesTextFormatSchemaOccurrences::Repeated => {
                    Err(ResponsesTextFormatRefusal::SchemaRepeated)
                }
            }),
        }
    }
}

pub enum ResponsesTextFormat {
    Text,
    JsonSchema { schema: Box<RawValue> },
}

impl<'de> Deserialize<'de> for ResponsesTextFormat {
    fn deserialize<TDeserializer>(deserializer: TDeserializer) -> Result<Self, TDeserializer::Error>
    where
        TDeserializer: Deserializer<'de>,
    {
        deserializer
            .deserialize_any(ResponsesTextFormatVisitor)?
            .map_err(ResponsesTextFormatRefusal::into_deserialization_error)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::from_str;

    use super::ResponsesTextFormat;

    #[derive(Debug, Eq, PartialEq)]
    enum DeserializedFormat {
        JsonSchema(String),
        Text,
    }

    fn deserialized(format: &str) -> Result<DeserializedFormat, String> {
        from_str::<ResponsesTextFormat>(format)
            .map(|deserialized_format| match deserialized_format {
                ResponsesTextFormat::JsonSchema { schema } => {
                    DeserializedFormat::JsonSchema(schema.get().to_owned())
                }
                ResponsesTextFormat::Text => DeserializedFormat::Text,
            })
            .map_err(|deserialization_error| deserialization_error.to_string())
    }

    #[test]
    fn keeps_the_schema_text_exactly_as_sent() {
        let schema = r#"{ "type": "integer", "maximum": 1e3 }"#;

        assert_eq!(
            deserialized(&format!(
                r#"{{"type": "json_schema", "name": "out", "schema": {schema}}}"#
            )),
            Ok(DeserializedFormat::JsonSchema(schema.to_owned()))
        );
    }

    #[test]
    fn accepts_the_array_form_of_every_format_type() {
        assert_eq!(
            [
                deserialized(r#"["text"]"#),
                deserialized(r#"["json_schema", {"type": "object"}]"#),
            ],
            [
                Ok(DeserializedFormat::Text),
                Ok(DeserializedFormat::JsonSchema(
                    r#"{"type": "object"}"#.to_owned()
                )),
            ]
        );
    }

    #[test]
    fn ignores_repeated_schemas_in_a_text_format() {
        assert_eq!(
            deserialized(r#"{"type": "text", "schema": {}, "schema": {}}"#),
            Ok(DeserializedFormat::Text)
        );
    }

    #[test]
    fn refuses_a_format_that_is_neither_an_object_nor_an_array() {
        assert_eq!(
            deserialized(r#""json_schema""#),
            Err(
                r#"invalid type: string "json_schema", expected internally tagged enum ResponsesTextFormat at line 1 column 13"#
                    .to_owned()
            )
        );
    }

    #[test]
    fn refuses_a_format_without_a_type() {
        assert_eq!(
            [deserialized(r#"{"schema": {}}"#), deserialized("[]")],
            [
                Err("missing field `type` at line 1 column 14".to_owned()),
                Err("missing field `type` at line 1 column 2".to_owned()),
            ]
        );
    }

    #[test]
    fn refuses_a_json_schema_format_without_a_schema() {
        assert_eq!(
            [
                deserialized(r#"{"type": "json_schema"}"#),
                deserialized(r#"["json_schema"]"#),
            ],
            [
                Err("missing field `schema`".to_owned()),
                Err(
                    "invalid length 0, expected struct variant ResponsesTextFormat::JsonSchema with 1 element"
                        .to_owned()
                ),
            ]
        );
    }

    #[test]
    fn refuses_a_format_that_repeats_its_type_or_its_json_schema() {
        assert_eq!(
            [
                deserialized(r#"{"type": "text", "type": "text"}"#),
                deserialized(r#"{"type": "json_schema", "schema": {}, "schema": {}}"#),
            ],
            [
                Err("duplicate field `type` at line 1 column 23".to_owned()),
                Err("duplicate field `schema`".to_owned()),
            ]
        );
    }

    #[test]
    fn refuses_an_array_format_with_elements_left_over() {
        assert_eq!(
            [
                deserialized(r#"["text", 1]"#),
                deserialized(r#"["json_schema", {}, 1]"#),
            ],
            [
                Err("invalid length 1, expected 0 elements in sequence".to_owned()),
                Err("invalid length 2, expected 1 element in sequence".to_owned()),
            ]
        );
    }

    #[test]
    fn refuses_a_format_whose_fields_are_malformed() {
        assert_eq!(
            [
                deserialized(r#"{"type": "text", 1: 2}"#),
                deserialized(r#"{"type": "json_schema", "schema": nul}"#),
                deserialized(r#"{"type": "text", "name": nul}"#),
            ],
            [
                Err("key must be a string at line 1 column 18".to_owned()),
                Err("expected ident at line 1 column 38".to_owned()),
                Err("expected ident at line 1 column 29".to_owned()),
            ]
        );
    }

    #[test]
    fn refuses_an_array_format_whose_elements_are_malformed() {
        assert_eq!(
            [
                deserialized("[nul]"),
                deserialized(r#"["json_schema", nul]"#),
                deserialized(r#"["text", nul]"#),
                deserialized(r#"["json_schema", {}, nul]"#),
            ],
            [
                Err("expected ident at line 1 column 5".to_owned()),
                Err("expected ident at line 1 column 20".to_owned()),
                Err("expected ident at line 1 column 13".to_owned()),
                Err("expected ident at line 1 column 24".to_owned()),
            ]
        );
    }
}
