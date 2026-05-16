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

#[cfg(test)]
mod tests {
    use super::SlotCountField;

    #[test]
    fn empty_input_resolves_to_empty_variant() {
        assert!(matches!(
            SlotCountField::from_user_input(String::new()),
            SlotCountField::Empty
        ));
    }

    #[test]
    fn positive_digit_input_resolves_to_valid_variant() {
        assert!(matches!(
            SlotCountField::from_user_input("42".to_owned()),
            SlotCountField::Valid { value: 42, .. }
        ));
    }

    #[test]
    fn zero_input_resolves_to_invalid_with_greater_than_zero_message() {
        assert!(matches!(
            SlotCountField::from_user_input("0".to_owned()),
            SlotCountField::Invalid { error, .. } if error.contains("greater than zero")
        ));
    }

    #[test]
    fn non_digit_input_resolves_to_invalid_with_generic_message() {
        assert!(matches!(
            SlotCountField::from_user_input("abc".to_owned()),
            SlotCountField::Invalid { error, .. } if error == "Invalid number of slots."
        ));
    }

    #[test]
    fn digit_input_overflowing_i32_resolves_to_too_large_message() {
        assert!(matches!(
            SlotCountField::from_user_input("9999999999".to_owned()),
            SlotCountField::Invalid { error, .. } if error == "Number of slots is too large."
        ));
    }

    #[test]
    fn raw_text_returns_inner_raw_for_valid_and_invalid_variants() {
        assert_eq!(SlotCountField::Empty.raw_text(), "");
        assert_eq!(
            SlotCountField::Valid {
                raw: "5".to_owned(),
                value: 5
            }
            .raw_text(),
            "5"
        );
        assert_eq!(
            SlotCountField::Invalid {
                raw: "abc".to_owned(),
                error: "Invalid number of slots.".to_owned()
            }
            .raw_text(),
            "abc"
        );
    }

    #[test]
    fn error_text_returns_error_only_for_invalid_variant() {
        assert!(SlotCountField::Empty.error_text().is_none());
        assert!(
            SlotCountField::Valid {
                raw: "5".to_owned(),
                value: 5
            }
            .error_text()
            .is_none()
        );
        assert_eq!(
            SlotCountField::Invalid {
                raw: "abc".to_owned(),
                error: "Invalid number of slots.".to_owned()
            }
            .error_text(),
            Some("Invalid number of slots.")
        );
    }

}
