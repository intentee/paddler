use tokio::time::Duration;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use crate::retry_delay_outcome::RetryDelayOutcome;

const RETRY_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Clone, Copy)]
pub enum RetryDelay {
    AfterRetryInterval,
    Immediately,
}

impl RetryDelay {
    pub async fn elapse(self, cancellation_token: &CancellationToken) -> RetryDelayOutcome {
        match self {
            Self::AfterRetryInterval => match cancellation_token
                .run_until_cancelled(sleep(RETRY_INTERVAL))
                .await
            {
                Some(()) => RetryDelayOutcome::Elapsed,
                None => RetryDelayOutcome::Cancelled,
            },
            Self::Immediately => RetryDelayOutcome::Elapsed,
        }
    }
}
