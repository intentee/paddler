use std::sync::Arc;

use async_stream::stream;
use futures::Stream;
use tokio::select;
use tokio_util::sync::CancellationToken;

use paddler_messaging::produces_snapshot::ProducesSnapshot;
use paddler_messaging::subscribes_to_updates::SubscribesToUpdates;

pub fn snapshots_stream<TProducer>(
    producer: Arc<TProducer>,
    shutdown: CancellationToken,
) -> impl Stream<Item = TProducer::Snapshot>
where
    TProducer: ProducesSnapshot + SubscribesToUpdates + Send + Sync + 'static,
    TProducer::Snapshot: Send + 'static,
{
    stream! {
        let mut update_rx = producer.subscribe_to_updates();

        loop {
            yield producer.make_snapshot();

            select! {
                () = shutdown.cancelled() => break,
                Ok(()) = update_rx.changed() => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use futures::StreamExt as _;
    use tokio_util::sync::CancellationToken;

    use paddler_messaging::buffered_request_manager_snapshot::BufferedRequestManagerSnapshot;
    use paddler_messaging::inference_mode::InferenceMode;

    use super::snapshots_stream;
    use crate::agent_controller_pool::AgentControllerPool;
    use crate::buffered_request_manager::BufferedRequestManager;

    fn buffered_request_manager() -> Arc<BufferedRequestManager> {
        Arc::new(BufferedRequestManager::new(
            Arc::new(AgentControllerPool::new(InferenceMode::TextGeneration)),
            Duration::MAX,
            1,
        ))
    }

    #[tokio::test]
    async fn snapshots_stream_emits_a_snapshot_after_every_update() {
        let buffered_request_manager = buffered_request_manager();
        let mut stream = Box::pin(snapshots_stream(
            buffered_request_manager.clone(),
            CancellationToken::new(),
        ));

        assert_eq!(
            stream.next().await,
            Some(BufferedRequestManagerSnapshot {
                buffered_requests_current: 0
            })
        );

        let _buffered_request = buffered_request_manager
            .buffered_request_counter
            .try_admit();

        assert_eq!(
            stream.next().await,
            Some(BufferedRequestManagerSnapshot {
                buffered_requests_current: 1
            })
        );
    }

    #[tokio::test]
    async fn snapshots_stream_terminates_on_shutdown() {
        let shutdown = CancellationToken::new();
        let mut stream = Box::pin(snapshots_stream(
            buffered_request_manager(),
            shutdown.clone(),
        ));

        assert_eq!(
            stream.next().await,
            Some(BufferedRequestManagerSnapshot {
                buffered_requests_current: 0
            })
        );

        shutdown.cancel();

        assert_eq!(stream.next().await, None);
    }
}
