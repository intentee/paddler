use serde_json::value::RawValue;

pub enum ResponsesTextFormatSchemaOccurrences {
    Absent,
    Once(Box<RawValue>),
    Repeated,
}

impl ResponsesTextFormatSchemaOccurrences {
    #[must_use]
    pub fn with_another(self, schema: Box<RawValue>) -> Self {
        match self {
            Self::Absent => Self::Once(schema),
            Self::Once(_) | Self::Repeated => Self::Repeated,
        }
    }
}
