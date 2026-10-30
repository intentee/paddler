use std::fmt;
use std::fmt::Formatter;

use serde::de::Error;
use serde::de::Expected;

const JSON_SCHEMA_FORMAT_ELEMENTS: &str =
    "struct variant ResponsesTextFormat::JsonSchema with 1 element";

struct ElementsInSequence {
    count: usize,
}

impl Expected for ElementsInSequence {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        if self.count == 1 {
            formatter.write_str("1 element in sequence")
        } else {
            write!(formatter, "{} elements in sequence", self.count)
        }
    }
}

pub enum ResponsesTextFormatRefusal {
    ElementsLeftOver {
        consumed_elements: usize,
        leftover_elements: usize,
    },
    SchemaElementMissing,
    SchemaMissing,
    SchemaRepeated,
}

impl ResponsesTextFormatRefusal {
    #[must_use]
    pub fn into_deserialization_error<TError>(self) -> TError
    where
        TError: Error,
    {
        match self {
            Self::ElementsLeftOver {
                consumed_elements,
                leftover_elements,
            } => TError::invalid_length(
                consumed_elements + leftover_elements,
                &ElementsInSequence {
                    count: consumed_elements,
                },
            ),
            Self::SchemaElementMissing => TError::invalid_length(0, &JSON_SCHEMA_FORMAT_ELEMENTS),
            Self::SchemaMissing => TError::missing_field("schema"),
            Self::SchemaRepeated => TError::duplicate_field("schema"),
        }
    }
}
