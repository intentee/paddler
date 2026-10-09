use actix_web::http::StatusCode;

use paddler_messaging::jsonrpc::error::Error as JsonRpcError;

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

impl From<&JsonRpcError> for UpstreamFailure {
    fn from(JsonRpcError { code, .. }: &JsonRpcError) -> Self {
        match code {
            503 => Self::Unavailable,
            504 => Self::TimedOut,
            _ => Self::RelayFailed,
        }
    }
}

#[cfg(test)]
mod tests {
    use actix_web::http::StatusCode;

    use paddler_messaging::jsonrpc::error::Error as JsonRpcError;

    use super::UpstreamFailure;

    #[test]
    fn maps_jsonrpc_codes_to_upstream_failures() {
        for (code, expected_failure) in [
            (503, UpstreamFailure::Unavailable),
            (504, UpstreamFailure::TimedOut),
            (500, UpstreamFailure::RelayFailed),
        ] {
            assert_eq!(
                UpstreamFailure::from(&JsonRpcError {
                    code,
                    description: "wire failure".to_owned(),
                }),
                expected_failure
            );
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
