use paddler_ports::bound_port::BoundPort;
use paddler_ports::check_optional_port::check_optional_port;
use paddler_ports::check_port::check_port;

#[derive(Default)]
pub enum AddressField {
    #[default]
    Empty,
    Bound { raw: String, port: BoundPort },
    Invalid { raw: String, error: String },
}

impl AddressField {
    #[must_use]
    pub fn required_from_user_input(raw: String) -> Self {
        match check_port(&raw) {
            Ok(port) => Self::Bound { raw, port },
            Err(error) => {
                if raw.is_empty() {
                    Self::Empty
                } else {
                    Self::Invalid {
                        raw,
                        error: error.user_facing_message(),
                    }
                }
            }
        }
    }

    #[must_use]
    pub fn optional_from_user_input(raw: String) -> Self {
        match check_optional_port(&raw) {
            Ok(Some(port)) => Self::Bound { raw, port },
            Ok(None) => Self::Empty,
            Err(error) => Self::Invalid {
                raw,
                error: error.user_facing_message(),
            },
        }
    }

    #[must_use]
    pub fn raw_text(&self) -> &str {
        match self {
            Self::Empty => "",
            Self::Bound { raw, .. } | Self::Invalid { raw, .. } => raw,
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
    use std::net::TcpListener;

    use super::AddressField;

    #[test]
    fn required_with_empty_input_resolves_to_empty() {
        assert!(matches!(
            AddressField::required_from_user_input(String::new()),
            AddressField::Empty
        ));
    }

    #[test]
    fn required_with_unparseable_input_resolves_to_invalid() {
        assert!(matches!(
            AddressField::required_from_user_input("not-a-socket".to_owned()),
            AddressField::Invalid { .. }
        ));
    }

    #[test]
    fn required_with_in_use_port_resolves_to_invalid() -> anyhow::Result<()> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let addr = listener.local_addr()?;

        assert!(matches!(
            AddressField::required_from_user_input(addr.to_string()),
            AddressField::Invalid { .. }
        ));
        Ok(())
    }

    #[test]
    fn optional_with_empty_input_resolves_to_empty() {
        assert!(matches!(
            AddressField::optional_from_user_input(String::new()),
            AddressField::Empty
        ));
    }

    #[test]
    fn optional_with_unparseable_input_resolves_to_invalid() {
        assert!(matches!(
            AddressField::optional_from_user_input("not-a-socket".to_owned()),
            AddressField::Invalid { .. }
        ));
    }

    #[test]
    fn optional_with_bindable_input_resolves_to_bound() -> anyhow::Result<()> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let address = listener.local_addr()?;
        drop(listener);

        assert!(matches!(
            AddressField::optional_from_user_input(address.to_string()),
            AddressField::Bound { .. }
        ));
        Ok(())
    }

    #[test]
    fn raw_text_returns_inner_raw_for_bound_and_invalid_variants() {
        assert_eq!(AddressField::Empty.raw_text(), "");
        assert_eq!(
            AddressField::Invalid {
                raw: "raw-text".to_owned(),
                error: "ignored".to_owned()
            }
            .raw_text(),
            "raw-text"
        );
    }

    #[test]
    fn error_text_returns_error_only_for_invalid_variant() {
        assert!(AddressField::Empty.error_text().is_none());
        assert_eq!(
            AddressField::Invalid {
                raw: String::new(),
                error: "the error".to_owned()
            }
            .error_text(),
            Some("the error")
        );
    }
}
