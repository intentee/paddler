use paddler_ports::bound_port::BoundPort;
use paddler_ports::check_optional_port::check_optional_port;
use paddler_ports::check_port::check_port;

pub enum AddressField {
    Empty,
    Bound { raw: String, port: BoundPort },
    Invalid { raw: String, error: String },
}

impl AddressField {
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

    pub fn raw_text(&self) -> &str {
        match self {
            Self::Empty => "",
            Self::Bound { raw, .. } | Self::Invalid { raw, .. } => raw,
        }
    }

    pub fn error_text(&self) -> Option<&str> {
        match self {
            Self::Invalid { error, .. } => Some(error),
            _ => None,
        }
    }
}

impl Default for AddressField {
    fn default() -> Self {
        Self::Empty
    }
}
