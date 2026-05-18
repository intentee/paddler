use std::net::SocketAddr;

#[derive(Default)]
pub enum ConnectAddressField {
    #[default]
    Empty,
    Valid {
        raw: String,
        socket_addr: SocketAddr,
    },
    Invalid {
        raw: String,
        error: String,
    },
}

impl ConnectAddressField {
    #[must_use]
    pub fn from_user_input(raw: String) -> Self {
        if raw.is_empty() {
            return Self::Empty;
        }

        match raw.parse::<SocketAddr>() {
            Ok(socket_addr) => Self::Valid { raw, socket_addr },
            Err(_) => Self::Invalid {
                raw,
                error: "Invalid address, expected format: IP:port".to_owned(),
            },
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
    use std::net::TcpListener;

    use super::ConnectAddressField;

    #[test]
    fn empty_input_resolves_to_empty() {
        assert!(matches!(
            ConnectAddressField::from_user_input(String::new()),
            ConnectAddressField::Empty
        ));
    }

    #[test]
    fn unparseable_input_resolves_to_invalid() {
        assert!(matches!(
            ConnectAddressField::from_user_input("not-a-socket".to_owned()),
            ConnectAddressField::Invalid { .. }
        ));
    }

    #[test]
    fn well_formed_input_resolves_to_valid_without_binding() {
        assert!(matches!(
            ConnectAddressField::from_user_input("127.0.0.1:8061".to_owned()),
            ConnectAddressField::Valid { .. }
        ));
    }

    #[test]
    fn an_address_currently_bound_by_another_process_is_still_accepted_as_valid()
    -> anyhow::Result<()> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let addr = listener.local_addr()?;

        assert!(matches!(
            ConnectAddressField::from_user_input(addr.to_string()),
            ConnectAddressField::Valid { .. }
        ));
        Ok(())
    }

    #[test]
    fn raw_text_returns_inner_raw_for_valid_and_invalid_variants() {
        assert_eq!(ConnectAddressField::Empty.raw_text(), "");
        assert_eq!(
            ConnectAddressField::Invalid {
                raw: "raw-text".to_owned(),
                error: "ignored".to_owned()
            }
            .raw_text(),
            "raw-text"
        );
    }

    #[test]
    fn error_text_returns_error_only_for_invalid_variant() {
        assert!(ConnectAddressField::Empty.error_text().is_none());
        assert_eq!(
            ConnectAddressField::Invalid {
                raw: String::new(),
                error: "the error".to_owned()
            }
            .error_text(),
            Some("the error")
        );
    }
}
