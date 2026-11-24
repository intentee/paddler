use std::fmt;
use std::fmt::Formatter;
use std::iter::from_fn;

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

fn remaining_elements<'de, TElement, TSeqAccess>(
    mut format_elements: TSeqAccess,
) -> Result<Vec<TElement>, TSeqAccess::Error>
where
    TElement: Deserialize<'de>,
    TSeqAccess: SeqAccess<'de>,
{
    from_fn(|| format_elements.next_element::<TElement>().transpose()).collect()
}

enum ResponsesTextFormatVisitOutcome {
    Accepted(ResponsesTextFormat),
    Refused(ResponsesTextFormatRefusal),
}

struct ResponsesTextFormatVisitor;

impl<'de> Visitor<'de> for ResponsesTextFormatVisitor {
    type Value = ResponsesTextFormatVisitOutcome;

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
            Some(ResponsesTextFormatType::Text) => {
                let leftover_elements = remaining_elements::<IgnoredAny, _>(format_elements)?;

                Ok(if leftover_elements.is_empty() {
                    ResponsesTextFormatVisitOutcome::Accepted(ResponsesTextFormat::Text)
                } else {
                    ResponsesTextFormatVisitOutcome::Refused(
                        ResponsesTextFormatRefusal::ElementsLeftOver {
                            expected_elements: 0,
                            received_elements: leftover_elements.len(),
                        },
                    )
                })
            }
            Some(ResponsesTextFormatType::JsonSchema) => {
                let schema_elements = remaining_elements::<Box<RawValue>, _>(format_elements)?;

                Ok(match <[Box<RawValue>; 1]>::try_from(schema_elements) {
                    Ok([schema]) => {
                        ResponsesTextFormatVisitOutcome::Accepted(ResponsesTextFormat::JsonSchema {
                            schema,
                        })
                    }
                    Err(schema_elements) if schema_elements.is_empty() => {
                        ResponsesTextFormatVisitOutcome::Refused(
                            ResponsesTextFormatRefusal::SchemaElementMissing,
                        )
                    }
                    Err(schema_elements) => ResponsesTextFormatVisitOutcome::Refused(
                        ResponsesTextFormatRefusal::ElementsLeftOver {
                            expected_elements: 1,
                            received_elements: schema_elements.len(),
                        },
                    ),
                })
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
            Some(ResponsesTextFormatType::Text) => Ok(ResponsesTextFormatVisitOutcome::Accepted(
                ResponsesTextFormat::Text,
            )),
            Some(ResponsesTextFormatType::JsonSchema) => Ok(match schema_occurrences {
                ResponsesTextFormatSchemaOccurrences::Absent => {
                    ResponsesTextFormatVisitOutcome::Refused(
                        ResponsesTextFormatRefusal::SchemaMissing,
                    )
                }
                ResponsesTextFormatSchemaOccurrences::Once(schema) => {
                    ResponsesTextFormatVisitOutcome::Accepted(ResponsesTextFormat::JsonSchema {
                        schema,
                    })
                }
                ResponsesTextFormatSchemaOccurrences::Repeated => {
                    ResponsesTextFormatVisitOutcome::Refused(
                        ResponsesTextFormatRefusal::SchemaRepeated,
                    )
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
        match deserializer.deserialize_any(ResponsesTextFormatVisitor)? {
            ResponsesTextFormatVisitOutcome::Accepted(responses_text_format) => {
                Ok(responses_text_format)
            }
            ResponsesTextFormatVisitOutcome::Refused(responses_text_format_refusal) => {
                Err(responses_text_format_refusal.into_deserialization_error())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserializer as _;
    use serde_json::Deserializer as JsonDeserializer;
    use serde_json::Error as SerdeJsonError;
    use serde_json::error::Category;
    use serde_json::from_str;

    use super::ResponsesTextFormat;
    use super::ResponsesTextFormatVisitOutcome;
    use super::ResponsesTextFormatVisitor;
    use crate::responses_text_format_refusal::ResponsesTextFormatRefusal;

    const ARRAY_FORMAT_WITHOUT_A_SCHEMA: &str = r#"["json_schema"]"#;
    const FORMAT_REPEATING_ITS_SCHEMA: &str =
        r#"{"type": "json_schema", "schema": {}, "schema": {}}"#;
    const FORMAT_WITHOUT_A_SCHEMA: &str = r#"{"type": "json_schema"}"#;
    const JSON_SCHEMA_FORMAT_WITH_AN_ELEMENT_LEFT_OVER: &str = r#"["json_schema", {}, 1]"#;
    const TEXT_FORMAT_WITH_AN_ELEMENT_LEFT_OVER: &str = r#"["text", 1]"#;

    fn deserialized(format: &str) -> Result<DeserializedFormat, DeserializationFailure> {
        from_str::<ResponsesTextFormat>(format)
            .map(|deserialized_format| match deserialized_format {
                ResponsesTextFormat::JsonSchema { schema } => {
                    DeserializedFormat::JsonSchema(schema.get().to_owned())
                }
                ResponsesTextFormat::Text => DeserializedFormat::Text,
            })
            .map_err(|deserialization_error| DeserializationFailure {
                category: deserialization_error.classify(),
                column: deserialization_error.column(),
                line: deserialization_error.line(),
            })
    }

    fn visited(format: &str) -> Result<ResponsesTextFormatVisitOutcome, SerdeJsonError> {
        JsonDeserializer::from_str(format).deserialize_any(ResponsesTextFormatVisitor)
    }

    #[derive(Debug, Eq, PartialEq)]
    struct DeserializationFailure {
        category: Category,
        column: usize,
        line: usize,
    }

    #[derive(Debug, Eq, PartialEq)]
    enum DeserializedFormat {
        JsonSchema(String),
        Text,
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
            Err(DeserializationFailure {
                category: Category::Data,
                column: 13,
                line: 1,
            })
        );
    }

    #[test]
    fn refuses_a_format_without_a_type() {
        assert_eq!(
            [deserialized(r#"{"schema": {}}"#), deserialized("[]")],
            [
                Err(DeserializationFailure {
                    category: Category::Data,
                    column: 14,
                    line: 1,
                }),
                Err(DeserializationFailure {
                    category: Category::Data,
                    column: 2,
                    line: 1,
                }),
            ]
        );
    }

    #[test]
    fn refuses_a_json_schema_format_without_a_schema() {
        assert!(matches!(
            visited(FORMAT_WITHOUT_A_SCHEMA),
            Ok(ResponsesTextFormatVisitOutcome::Refused(
                ResponsesTextFormatRefusal::SchemaMissing
            ))
        ));
        assert!(matches!(
            visited(ARRAY_FORMAT_WITHOUT_A_SCHEMA),
            Ok(ResponsesTextFormatVisitOutcome::Refused(
                ResponsesTextFormatRefusal::SchemaElementMissing
            ))
        ));
    }

    #[test]
    fn refuses_a_format_that_repeats_its_type_or_its_json_schema() {
        assert_eq!(
            deserialized(r#"{"type": "text", "type": "text"}"#),
            Err(DeserializationFailure {
                category: Category::Data,
                column: 23,
                line: 1,
            })
        );
        assert!(matches!(
            visited(FORMAT_REPEATING_ITS_SCHEMA),
            Ok(ResponsesTextFormatVisitOutcome::Refused(
                ResponsesTextFormatRefusal::SchemaRepeated
            ))
        ));
    }

    #[test]
    fn refuses_an_array_format_with_elements_left_over() {
        assert!(matches!(
            visited(TEXT_FORMAT_WITH_AN_ELEMENT_LEFT_OVER),
            Ok(ResponsesTextFormatVisitOutcome::Refused(
                ResponsesTextFormatRefusal::ElementsLeftOver {
                    expected_elements: 0,
                    received_elements: 1,
                }
            ))
        ));
        assert!(matches!(
            visited(JSON_SCHEMA_FORMAT_WITH_AN_ELEMENT_LEFT_OVER),
            Ok(ResponsesTextFormatVisitOutcome::Refused(
                ResponsesTextFormatRefusal::ElementsLeftOver {
                    expected_elements: 1,
                    received_elements: 2,
                }
            ))
        ));
    }

    #[test]
    fn surfaces_every_refusal_as_a_data_error() {
        for refused_format in [
            ARRAY_FORMAT_WITHOUT_A_SCHEMA,
            FORMAT_REPEATING_ITS_SCHEMA,
            FORMAT_WITHOUT_A_SCHEMA,
            JSON_SCHEMA_FORMAT_WITH_AN_ELEMENT_LEFT_OVER,
            TEXT_FORMAT_WITH_AN_ELEMENT_LEFT_OVER,
        ] {
            assert!(matches!(
                deserialized(refused_format),
                Err(DeserializationFailure {
                    category: Category::Data,
                    ..
                })
            ));
        }
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
                Err(DeserializationFailure {
                    category: Category::Syntax,
                    column: 18,
                    line: 1,
                }),
                Err(DeserializationFailure {
                    category: Category::Syntax,
                    column: 38,
                    line: 1,
                }),
                Err(DeserializationFailure {
                    category: Category::Syntax,
                    column: 29,
                    line: 1,
                }),
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
                Err(DeserializationFailure {
                    category: Category::Syntax,
                    column: 5,
                    line: 1,
                }),
                Err(DeserializationFailure {
                    category: Category::Syntax,
                    column: 20,
                    line: 1,
                }),
                Err(DeserializationFailure {
                    category: Category::Syntax,
                    column: 13,
                    line: 1,
                }),
                Err(DeserializationFailure {
                    category: Category::Syntax,
                    column: 24,
                    line: 1,
                }),
            ]
        );
    }
}
