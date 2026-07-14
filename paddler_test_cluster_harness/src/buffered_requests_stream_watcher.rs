use std::pin::Pin;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use futures_util::Stream;
use futures_util::StreamExt as _;
use paddler_client::client_management::ClientManagement;
use paddler_messaging::buffered_request_manager_snapshot::BufferedRequestManagerSnapshot;

pub struct BufferedRequestsStreamWatcher {
    stream: Pin<Box<dyn Stream<Item = Result<BufferedRequestManagerSnapshot>> + Send>>,
}

impl BufferedRequestsStreamWatcher {
    pub async fn connect(management: &ClientManagement) -> Result<Self> {
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

#[cfg(test)]
mod tests {
    use anyhow::anyhow;
    use paddler_messaging::buffered_request_manager_snapshot::BufferedRequestManagerSnapshot;

    use super::BufferedRequestsStreamWatcher;

    fn watcher(
        items: Vec<anyhow::Result<BufferedRequestManagerSnapshot>>,
    ) -> BufferedRequestsStreamWatcher {
        BufferedRequestsStreamWatcher::from_stream(Box::pin(futures_util::stream::iter(items)))
    }

    fn snapshot(buffered_requests_current: i32) -> BufferedRequestManagerSnapshot {
        BufferedRequestManagerSnapshot {
            buffered_requests_current,
        }
    }

    #[tokio::test]
    async fn returns_the_first_snapshot_satisfying_the_predicate() {
        let mut watcher = watcher(vec![Ok(snapshot(5)), Ok(snapshot(0))]);

        let matched = watcher
            .until(|snapshot| snapshot.buffered_requests_current == 0)
            .await
            .unwrap();

        assert_eq!(matched.buffered_requests_current, 0);
    }

    #[tokio::test]
    async fn errors_when_the_stream_closes_before_the_predicate_is_satisfied() {
        let mut watcher = watcher(vec![Ok(snapshot(5))]);

        let error = watcher.until(|_| false).await.err().unwrap();

        assert!(error.to_string().contains("closed before predicate"));
    }

    #[tokio::test]
    async fn propagates_a_stream_error() {
        let mut watcher = watcher(vec![Err(anyhow!("socket closed"))]);

        let error = watcher.until(|_| true).await.err().unwrap();

        assert!(error.to_string().contains("yielded an error"));
    }
}
