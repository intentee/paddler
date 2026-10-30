use std::future::Future;
use std::pin::Pin;

use tokio_util::sync::CancellationToken;

pub struct StartupPhase<TOutcome> {
    pub cancellation_token: CancellationToken,
    pub outcome: Pin<Box<dyn Future<Output = TOutcome> + Send>>,
}
