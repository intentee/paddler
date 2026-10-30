use actix_web::http::StatusCode;

use crate::relay_failure::RelayFailure;

#[derive(Debug, Eq, PartialEq)]
pub enum UpstreamFailure {
    AgentFailed,
    RelayFailed,
    TimedOut,
    Unavailable,
}

impl UpstreamFailure {
    #[must_use]
    pub const fn status_code(&self) -> StatusCode {
        match self {
            Self::AgentFailed => StatusCode::INTERNAL_SERVER_ERROR,
            Self::RelayFailed => StatusCode::BAD_GATEWAY,
            Self::TimedOut => StatusCode::GATEWAY_TIMEOUT,
            Self::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
}

impl From<&RelayFailure> for UpstreamFailure {
    fn from(relay_failure: &RelayFailure) -> Self {
        match relay_failure {
            RelayFailure::AgentDisconnected => Self::RelayFailed,
            RelayFailure::BalancerShuttingDown | RelayFailure::BufferOverflow => Self::Unavailable,
            RelayFailure::BufferedRequestTimedOut | RelayFailure::ItemTimedOut { .. } => {
                Self::TimedOut
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use actix_web::http::StatusCode;

    use super::UpstreamFailure;
    use crate::relay_failure::RelayFailure;

    #[test]
    fn maps_relay_failures_to_upstream_failures() {
        for (relay_failure, expected_failure) in [
            (
                RelayFailure::AgentDisconnected,
                UpstreamFailure::RelayFailed,
            ),
            (
                RelayFailure::BalancerShuttingDown,
                UpstreamFailure::Unavailable,
            ),
            (RelayFailure::BufferOverflow, UpstreamFailure::Unavailable),
            (
                RelayFailure::BufferedRequestTimedOut,
                UpstreamFailure::TimedOut,
            ),
            (
                RelayFailure::ItemTimedOut {
                    inference_item_timeout: Duration::from_millis(250),
                },
                UpstreamFailure::TimedOut,
            ),
        ] {
            assert_eq!(UpstreamFailure::from(&relay_failure), expected_failure);
        }
    }

    #[test]
    fn maps_upstream_failures_to_http_statuses() {
        for (failure, expected_status) in [
            (
                UpstreamFailure::AgentFailed,
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            (UpstreamFailure::RelayFailed, StatusCode::BAD_GATEWAY),
            (UpstreamFailure::TimedOut, StatusCode::GATEWAY_TIMEOUT),
            (
                UpstreamFailure::Unavailable,
                StatusCode::SERVICE_UNAVAILABLE,
            ),
        ] {
            assert_eq!(failure.status_code(), expected_status);
        }
    }
}
