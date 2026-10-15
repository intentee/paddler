use tokio::time::Duration;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

const RETRY_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Clone, Copy)]
pub enum RetryDelay {
    AfterRetryInterval,
    Immediately,
}

impl RetryDelay {
    pub async fn elapse(self, cancellation_token: &CancellationToken) {
        match self {
            Self::AfterRetryInterval => {
                cancellation_token
                    .run_until_cancelled(sleep(RETRY_INTERVAL))
                    .await;
            }
            Self::Immediately => {}
        }
    }
}
