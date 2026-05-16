use std::num::IntErrorKind;

#[derive(Default)]
pub enum SlotCountField {
    #[default]
    Empty,
    Valid { raw: String, value: i32 },
    Invalid { raw: String, error: String },
}

impl SlotCountField {
    #[must_use]
    pub fn from_user_input(raw: String) -> Self {
        if raw.is_empty() {
            return Self::Empty;
        }

        if !raw.chars().all(|character| character.is_ascii_digit()) {
            return Self::Invalid {
                raw,
                error: "Invalid number of slots.".to_owned(),
            };
        }

        match raw.parse::<i32>() {
            Ok(value) if value > 0 => Self::Valid { raw, value },
            Ok(_) => Self::Invalid {
                raw,
                error: "Invalid number of slots (the number should be greater than zero)."
                    .to_owned(),
            },
            Err(error) => {
                let message = match error.kind() {
                    IntErrorKind::PosOverflow => "Number of slots is too large.",
                    _ => "Invalid number of slots.",
                };
                Self::Invalid {
                    raw,
                    error: message.to_owned(),
                }
            }
        }
    }

    #[must_use]
    pub fn raw_text(&self) -> &str {
        match self {
            Self::Empty => "",
            Self::Valid { raw, .. } | Self::Invalid { raw, .. } => raw,
        }
    }

    #[must_use]
    pub fn error_text(&self) -> Option<&str> {
        match self {
            Self::Invalid { error, .. } => Some(error),
            _ => None,
        }
    }
}
