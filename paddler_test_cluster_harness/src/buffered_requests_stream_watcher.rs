use std::pin::Pin;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use futures_util::Stream;
use futures_util::StreamExt as _;
use paddler_client::client_management::ClientManagement;
use paddler_messaging::buffered_request_manager_snapshot::BufferedRequestManagerSnapshot;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

use crate::observation_window::ObservationWindow;

pub struct BufferedRequestsStreamWatcher {
    stream: Pin<Box<dyn Stream<Item = Result<BufferedRequestManagerSnapshot>> + Send>>,
}

impl BufferedRequestsStreamWatcher {
    pub async fn connect(
        cancellation_token: CancellationToken,
        management: &ClientManagement,
    ) -> Result<Self> {
        let raw_stream = management
            .get_buffered_requests_stream(cancellation_token)
            .await
            .map_err(anyhow::Error::new)
            .context("failed to open /api/v1/buffered_requests/stream")?;

        let stream = raw_stream.map(|item| item.map_err(anyhow::Error::new));

        Ok(Self {
            stream: Box::pin(stream),
        })
    }

    pub async fn until<TPredicate>(
        &mut self,
        observation_window: ObservationWindow,
        mut predicate: TPredicate,
    ) -> Result<BufferedRequestManagerSnapshot>
    where
        TPredicate: FnMut(&BufferedRequestManagerSnapshot) -> bool,
    {
        let stream = &mut self.stream;

        timeout(observation_window.duration(), async move {
            while let Some(item) = stream.next().await {
                let snapshot = item.context("buffered requests stream yielded an error")?;

                if predicate(&snapshot) {
                    return Ok(snapshot);
                }
            }

            Err(anyhow!(
                "buffered requests stream closed before predicate was satisfied"
            ))
        })
        .await
        .with_context(|| {
            format!(
                "buffered requests stream did not satisfy the predicate within {:?}",
                observation_window.duration()
            )
        })?
    }
}
