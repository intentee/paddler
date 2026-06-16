use std::pin::Pin;

use anyhow::Context as _;
use anyhow::Result;
use futures_util::Stream;
use futures_util::StreamExt as _;
use paddler_messaging::buffered_request_manager_snapshot::BufferedRequestManagerSnapshot;

use paddler_client::management_client::ManagementClient;

use crate::next_matching_snapshot::next_matching_snapshot;

pub struct BufferedRequestsStreamWatcher {
    stream: Pin<Box<dyn Stream<Item = Result<BufferedRequestManagerSnapshot>> + Send>>,
}

impl BufferedRequestsStreamWatcher {
    pub async fn connect(management: &ManagementClient) -> Result<Self> {
        let raw_stream = management
            .buffered_requests_stream()
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
        predicate: TPredicate,
    ) -> Result<BufferedRequestManagerSnapshot>
    where
        TPredicate: FnMut(&BufferedRequestManagerSnapshot) -> bool,
    {
        next_matching_snapshot(&mut self.stream, predicate).await
    }
}
