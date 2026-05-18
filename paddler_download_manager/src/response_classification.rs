use reqwest::StatusCode;

#[derive(Debug, Eq, PartialEq)]
pub enum ResponseClassification {
    NotFound,
    PartialFileStale,
    PermissionDenied(StatusCode),
    Retryable(StatusCode),
    StreamFromCurrentOffset,
    StreamFromStart,
    StreamFromStartIgnoringRange,
}

impl ResponseClassification {
    #[must_use]
    pub fn from_status(status: StatusCode, sent_range_header: bool) -> Self {
        if status == StatusCode::PARTIAL_CONTENT {
            return Self::StreamFromCurrentOffset;
        }

        if status == StatusCode::OK {
            if sent_range_header {
                return Self::StreamFromStartIgnoringRange;
            }
            return Self::StreamFromStart;
        }

        if status == StatusCode::NOT_FOUND {
            return Self::NotFound;
        }

        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            return Self::PermissionDenied(status);
        }

        if status == StatusCode::RANGE_NOT_SATISFIABLE {
            return Self::PartialFileStale;
        }

        Self::Retryable(status)
    }
}

#[cfg(test)]
mod tests {
    use reqwest::StatusCode;

    use crate::response_classification::ResponseClassification;

    #[test]
    fn from_status_206_returns_stream_from_current_offset() {
        assert_eq!(
            ResponseClassification::from_status(StatusCode::PARTIAL_CONTENT, true),
            ResponseClassification::StreamFromCurrentOffset
        );
    }

    #[test]
    fn from_status_200_on_range_request_returns_stream_from_start_ignoring_range() {
        assert_eq!(
            ResponseClassification::from_status(StatusCode::OK, true),
            ResponseClassification::StreamFromStartIgnoringRange
        );
    }

    #[test]
    fn from_status_200_on_plain_request_returns_stream_from_start() {
        assert_eq!(
            ResponseClassification::from_status(StatusCode::OK, false),
            ResponseClassification::StreamFromStart
        );
    }

    #[test]
    fn from_status_404_returns_not_found() {
        assert_eq!(
            ResponseClassification::from_status(StatusCode::NOT_FOUND, false),
            ResponseClassification::NotFound
        );
    }

    #[test]
    fn from_status_401_returns_permission_denied() {
        assert_eq!(
            ResponseClassification::from_status(StatusCode::UNAUTHORIZED, false),
            ResponseClassification::PermissionDenied(StatusCode::UNAUTHORIZED)
        );
    }

    #[test]
    fn from_status_403_returns_permission_denied() {
        assert_eq!(
            ResponseClassification::from_status(StatusCode::FORBIDDEN, false),
            ResponseClassification::PermissionDenied(StatusCode::FORBIDDEN)
        );
    }

    #[test]
    fn from_status_416_returns_partial_file_stale() {
        assert_eq!(
            ResponseClassification::from_status(StatusCode::RANGE_NOT_SATISFIABLE, true),
            ResponseClassification::PartialFileStale
        );
    }

    #[test]
    fn from_status_503_returns_retryable() {
        assert_eq!(
            ResponseClassification::from_status(StatusCode::SERVICE_UNAVAILABLE, false),
            ResponseClassification::Retryable(StatusCode::SERVICE_UNAVAILABLE)
        );
    }

    #[test]
    fn from_status_500_returns_retryable() {
        assert_eq!(
            ResponseClassification::from_status(StatusCode::INTERNAL_SERVER_ERROR, false),
            ResponseClassification::Retryable(StatusCode::INTERNAL_SERVER_ERROR)
        );
    }
}
