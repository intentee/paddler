use paddler_download_manager::download_error::DownloadError;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;

#[must_use]
pub const fn download_error_agent_issue(
    error: &DownloadError,
    model_path: ModelPath,
) -> AgentIssue {
    match error {
        DownloadError::InvalidUrl { .. } | DownloadError::UnsupportedUrlScheme { .. } => {
            AgentIssue::DownloadUrlIsMalformed(model_path)
        }
        DownloadError::NotFound { .. } => AgentIssue::ModelDoesNotExistAtUrl(model_path),
        DownloadError::PermissionDenied { .. } => {
            AgentIssue::DownloadServerDeniedAccess(model_path)
        }
        DownloadError::DownloadServerIsUnreachable { .. } => {
            AgentIssue::DownloadServerIsUnreachable(model_path)
        }
        DownloadError::DownloadServerErrored { .. } => {
            AgentIssue::DownloadServerErrored(model_path)
        }
        DownloadError::DownloadServerRejectedRequest { .. } => {
            AgentIssue::DownloadServerRejectedRequest(model_path)
        }
        DownloadError::DownloadInterrupted { .. } => AgentIssue::DownloadInterrupted(model_path),
        DownloadError::CachePermissionDenied { .. } => {
            AgentIssue::CacheDirectoryIsNotWritable(model_path)
        }
        DownloadError::CacheDiskFull { .. } => AgentIssue::CacheStorageIsFull(model_path),
        DownloadError::PartialFileStale { .. }
        | DownloadError::PartialPathIsADirectory { .. }
        | DownloadError::FinalPathHasNoFileName { .. }
        | DownloadError::Io { .. } => AgentIssue::ModelCacheIsCorrupted(model_path),
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::PathBuf;

    use reqwest::Client;
    use reqwest::Error as ReqwestError;
    use reqwest::StatusCode;
    use url::Url;

    use paddler_download_manager::download_error::DownloadError;
    use paddler_messaging::agent_issue::AgentIssue;
    use paddler_messaging::agent_issue_params::model_path::ModelPath;

    use super::download_error_agent_issue;

    const TEST_URL: &str = "https://example.com/m.gguf";

    fn test_model_path() -> ModelPath {
        ModelPath {
            model_path: TEST_URL.to_owned(),
        }
    }

    async fn refused_connection_error() -> ReqwestError {
        Client::new()
            .get("http://127.0.0.1:1/model.gguf")
            .send()
            .await
            .expect_err("nothing listens on the loopback discard port")
    }

    #[test]
    fn invalid_url_maps_to_download_url_is_malformed() {
        let parse_error = Url::parse("not a url").err().unwrap();
        let error = DownloadError::InvalidUrl {
            url: "not a url".to_owned(),
            source: parse_error,
        };

        assert_eq!(
            download_error_agent_issue(&error, test_model_path()),
            AgentIssue::DownloadUrlIsMalformed(test_model_path())
        );
    }

    #[test]
    fn unsupported_url_scheme_maps_to_download_url_is_malformed() {
        let error = DownloadError::UnsupportedUrlScheme {
            url: TEST_URL.to_owned(),
            scheme: "ftp".to_owned(),
        };

        assert_eq!(
            download_error_agent_issue(&error, test_model_path()),
            AgentIssue::DownloadUrlIsMalformed(test_model_path())
        );
    }

    #[test]
    fn not_found_maps_to_model_does_not_exist_at_url() {
        let error = DownloadError::NotFound {
            url: TEST_URL.to_owned(),
        };

        assert_eq!(
            download_error_agent_issue(&error, test_model_path()),
            AgentIssue::ModelDoesNotExistAtUrl(test_model_path())
        );
    }

    #[test]
    fn permission_denied_maps_to_download_server_denied_access() {
        let error = DownloadError::PermissionDenied {
            url: TEST_URL.to_owned(),
            status: StatusCode::FORBIDDEN,
        };

        assert_eq!(
            download_error_agent_issue(&error, test_model_path()),
            AgentIssue::DownloadServerDeniedAccess(test_model_path())
        );
    }

    #[test]
    fn partial_file_stale_maps_to_model_cache_is_corrupted() {
        let error = DownloadError::PartialFileStale {
            url: TEST_URL.to_owned(),
            partial_path: PathBuf::from("/tmp/stale.partial"),
        };

        assert_eq!(
            download_error_agent_issue(&error, test_model_path()),
            AgentIssue::ModelCacheIsCorrupted(test_model_path())
        );
    }

    #[tokio::test]
    async fn download_server_is_unreachable_maps_to_agent_issue() {
        let error = DownloadError::DownloadServerIsUnreachable {
            url: TEST_URL.to_owned(),
            source: refused_connection_error().await,
        };

        assert_eq!(
            download_error_agent_issue(&error, test_model_path()),
            AgentIssue::DownloadServerIsUnreachable(test_model_path())
        );
    }

    #[test]
    fn download_server_errored_maps_to_agent_issue() {
        let error = DownloadError::DownloadServerErrored {
            url: TEST_URL.to_owned(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        };

        assert_eq!(
            download_error_agent_issue(&error, test_model_path()),
            AgentIssue::DownloadServerErrored(test_model_path())
        );
    }

    #[test]
    fn download_server_rejected_request_maps_to_agent_issue() {
        let error = DownloadError::DownloadServerRejectedRequest {
            url: TEST_URL.to_owned(),
            status: StatusCode::BAD_REQUEST,
        };

        assert_eq!(
            download_error_agent_issue(&error, test_model_path()),
            AgentIssue::DownloadServerRejectedRequest(test_model_path())
        );
    }

    #[tokio::test]
    async fn download_interrupted_maps_to_agent_issue() {
        let error = DownloadError::DownloadInterrupted {
            url: TEST_URL.to_owned(),
            source: refused_connection_error().await,
        };

        assert_eq!(
            download_error_agent_issue(&error, test_model_path()),
            AgentIssue::DownloadInterrupted(test_model_path())
        );
    }

    #[test]
    fn cache_permission_denied_maps_to_cache_directory_is_not_writable() {
        let error = DownloadError::CachePermissionDenied {
            path: PathBuf::from("/tmp/locked/model.partial"),
            source: io::Error::from(io::ErrorKind::PermissionDenied),
        };

        assert_eq!(
            download_error_agent_issue(&error, test_model_path()),
            AgentIssue::CacheDirectoryIsNotWritable(test_model_path())
        );
    }

    #[test]
    fn cache_disk_full_maps_to_cache_storage_is_full() {
        let error = DownloadError::CacheDiskFull {
            path: PathBuf::from("/tmp/full/model.partial"),
            source: io::Error::from(io::ErrorKind::StorageFull),
        };

        assert_eq!(
            download_error_agent_issue(&error, test_model_path()),
            AgentIssue::CacheStorageIsFull(test_model_path())
        );
    }

    #[test]
    fn io_maps_to_model_cache_is_corrupted() {
        let error = DownloadError::Io {
            path: PathBuf::from("/tmp/anywhere/model.partial"),
            source: io::Error::from(io::ErrorKind::NotFound),
        };

        assert_eq!(
            download_error_agent_issue(&error, test_model_path()),
            AgentIssue::ModelCacheIsCorrupted(test_model_path())
        );
    }
}
