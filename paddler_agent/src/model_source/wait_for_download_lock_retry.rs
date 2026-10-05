use tokio::time::Duration;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use crate::model_source::download_lock_retry_error::DownloadLockRetryError;

pub async fn wait_for_download_lock_retry(
    cancellation_token: &CancellationToken,
    lock_retry_timeout: Duration,
    lock_path: String,
    model_path: String,
) -> DownloadLockRetryError {
    match cancellation_token
        .run_until_cancelled(sleep(lock_retry_timeout))
        .await
    {
        None => DownloadLockRetryError::Cancelled {
            lock_path,
            model_path,
        },
        Some(()) => DownloadLockRetryError::LockStillUnavailable { lock_path },
    }
}

#[cfg(test)]
mod tests {
    use tokio::time::Duration;
    use tokio_util::sync::CancellationToken;

    use super::wait_for_download_lock_retry;
    use crate::model_source::download_lock_retry_error::DownloadLockRetryError;

    const LOCK_PATH: &str = "/tmp/model.lock";
    const MODEL_PATH: &str = "repo/main/model.gguf";
    const RETRY_TIMEOUT_THAT_ELAPSES_IMMEDIATELY: Duration = Duration::ZERO;
    const RETRY_TIMEOUT_THAT_OUTLIVES_THE_TEST: Duration = Duration::from_hours(1);

    #[tokio::test]
    async fn reports_cancelled_when_the_token_is_cancelled_before_the_wait_elapses() {
        let cancellation_token = CancellationToken::new();

        cancellation_token.cancel();

        assert!(matches!(
            wait_for_download_lock_retry(
                &cancellation_token,
                RETRY_TIMEOUT_THAT_OUTLIVES_THE_TEST,
                LOCK_PATH.to_owned(),
                MODEL_PATH.to_owned(),
            )
            .await,
            DownloadLockRetryError::Cancelled { .. }
        ));
    }

    #[tokio::test]
    async fn reports_the_lock_is_still_unavailable_when_the_wait_elapses() {
        assert!(matches!(
            wait_for_download_lock_retry(
                &CancellationToken::new(),
                RETRY_TIMEOUT_THAT_ELAPSES_IMMEDIATELY,
                LOCK_PATH.to_owned(),
                MODEL_PATH.to_owned(),
            )
            .await,
            DownloadLockRetryError::LockStillUnavailable { .. }
        ));
    }
}
