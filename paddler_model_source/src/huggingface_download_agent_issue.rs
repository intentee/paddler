use hf_hub::HFError;

use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::hugging_face_download_failed_params::HuggingFaceDownloadFailedParams;
use paddler_messaging::agent_issue_params::hugging_face_download_failure::HuggingFaceDownloadFailure;
use paddler_messaging::agent_issue_params::hugging_face_download_lock::HuggingFaceDownloadLock;
use paddler_messaging::agent_issue_params::model_path::ModelPath;

fn hugging_face_download_failure(download_error: &HFError) -> HuggingFaceDownloadFailure {
    match download_error {
        HFError::Http { context }
        | HFError::Conflict { context }
        | HFError::RateLimited { context, .. } => {
            HuggingFaceDownloadFailure::HubRespondedWithStatus {
                status: context.status.as_u16(),
            }
        }
        HFError::Request { .. } => HuggingFaceDownloadFailure::HubConnectionFailed,
        HFError::Io(_) => HuggingFaceDownloadFailure::CacheIoFailed,
        HFError::Json(_) | HFError::MalformedResponse { .. } => {
            HuggingFaceDownloadFailure::HubResponseMalformed
        }
        HFError::Xet { .. } => HuggingFaceDownloadFailure::XetTransferFailed,
        _ => HuggingFaceDownloadFailure::UnexpectedHubClientFailure,
    }
}

#[must_use]
pub fn huggingface_download_agent_issue(
    download_error: &HFError,
    model_path: ModelPath,
) -> AgentIssue {
    match download_error {
        HFError::EntryNotFound { .. }
        | HFError::RepoNotFound { .. }
        | HFError::RevisionNotFound { .. } => AgentIssue::HuggingFaceModelDoesNotExist(model_path),
        HFError::AuthRequired { .. } | HFError::Forbidden { .. } => {
            AgentIssue::HuggingFacePermissions(model_path)
        }
        HFError::CacheLockTimeout { path } => {
            AgentIssue::HuggingFaceCannotAcquireLock(HuggingFaceDownloadLock {
                lock_path: path.display().to_string(),
                model_path,
            })
        }
        other_download_error => {
            AgentIssue::HuggingFaceDownloadFailed(HuggingFaceDownloadFailedParams {
                failure: hugging_face_download_failure(other_download_error),
                model_path,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::io::ErrorKind;
    use std::path::PathBuf;

    use hf_hub::HFError;
    use hf_hub::XetOperation;
    use reqwest::get;

    use paddler_messaging::agent_issue::AgentIssue;
    use paddler_messaging::agent_issue_params::hugging_face_download_failed_params::HuggingFaceDownloadFailedParams;
    use paddler_messaging::agent_issue_params::hugging_face_download_failure::HuggingFaceDownloadFailure;
    use paddler_messaging::agent_issue_params::hugging_face_download_lock::HuggingFaceDownloadLock;
    use paddler_messaging::agent_issue_params::model_path::ModelPath;

    use super::huggingface_download_agent_issue;

    fn model_path() -> ModelPath {
        ModelPath {
            model_path: "owner/repo/model.gguf".to_owned(),
        }
    }

    fn download_failure(failure: HuggingFaceDownloadFailure) -> AgentIssue {
        AgentIssue::HuggingFaceDownloadFailed(HuggingFaceDownloadFailedParams {
            failure,
            model_path: model_path(),
        })
    }

    #[test]
    fn a_timed_out_cache_lock_is_reported_with_its_lock_path() {
        assert_eq!(
            huggingface_download_agent_issue(
                &HFError::CacheLockTimeout {
                    path: PathBuf::from("/cache/.locks/model.lock"),
                },
                model_path(),
            ),
            AgentIssue::HuggingFaceCannotAcquireLock(HuggingFaceDownloadLock {
                lock_path: "/cache/.locks/model.lock".to_owned(),
                model_path: model_path(),
            })
        );
    }

    #[tokio::test]
    async fn a_hub_the_client_cannot_reach_is_a_failed_connection() {
        let unreachable_hub_error = get("http://127.0.0.1:1/").await.unwrap_err();

        assert_eq!(
            huggingface_download_agent_issue(&HFError::from(unreachable_hub_error), model_path()),
            download_failure(HuggingFaceDownloadFailure::HubConnectionFailed)
        );
    }

    #[test]
    fn a_cache_file_operation_failure_is_a_cache_io_failure() {
        assert_eq!(
            huggingface_download_agent_issue(
                &HFError::Io(io::Error::from(ErrorKind::StorageFull)),
                model_path(),
            ),
            download_failure(HuggingFaceDownloadFailure::CacheIoFailed)
        );
    }

    #[test]
    fn a_failed_xet_transfer_is_reported_as_such() {
        assert_eq!(
            huggingface_download_agent_issue(
                &HFError::xet(
                    XetOperation::Download,
                    io::Error::from(ErrorKind::ConnectionReset),
                ),
                model_path(),
            ),
            download_failure(HuggingFaceDownloadFailure::XetTransferFailed)
        );
    }

    #[test]
    fn a_client_failure_paddler_does_not_expect_is_reported_as_unexpected() {
        assert_eq!(
            huggingface_download_agent_issue(
                &HFError::InvalidParameter("revision".to_owned()),
                model_path(),
            ),
            download_failure(HuggingFaceDownloadFailure::UnexpectedHubClientFailure)
        );
    }
}
