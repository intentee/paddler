use std::pin::Pin;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use futures_util::Stream;
use futures_util::StreamExt as _;
use paddler_client::ClientManagement;
use paddler_types::buffered_request_manager_snapshot::BufferedRequestManagerSnapshot;

pub struct BufferedRequestsStreamWatcher {
    stream: Pin<Box<dyn Stream<Item = Result<BufferedRequestManagerSnapshot>> + Send>>,
}

impl BufferedRequestsStreamWatcher {
    pub async fn connect(management: &ClientManagement<'_>) -> Result<Self> {
        let raw_stream = management
            .get_buffered_requests_stream()
            .await
            .map_err(anyhow::Error::new)
            .context("failed to open /api/v1/buffered_requests/stream")?;

        let stream = raw_stream.map(|item| item.map_err(anyhow::Error::new));

        Ok(Self {
            stream: Box::pin(stream),
        })
    }

    #[must_use]
    pub fn from_stream(
        stream: Pin<Box<dyn Stream<Item = Result<BufferedRequestManagerSnapshot>> + Send>>,
    ) -> Self {
        Self { stream }
    }

    pub async fn until<TPredicate>(
        &mut self,
        mut predicate: TPredicate,
    ) -> Result<BufferedRequestManagerSnapshot>
    where
        TPredicate: FnMut(&BufferedRequestManagerSnapshot) -> bool,
    {
        while let Some(item) = self.stream.next().await {
            let snapshot = item.context("buffered requests stream yielded an error")?;

            if predicate(&snapshot) {
                return Ok(snapshot);
            }
        }

        Err(anyhow!(
            "buffered requests stream closed before predicate was satisfied"
        ))
    }
}
