use std::sync::Arc;

use async_stream::stream;
use futures::Stream;
use log::error;
use tokio_util::sync::CancellationToken;

use crate::produces_snapshot::ProducesSnapshot;
use crate::subscribes_to_updates::SubscribesToUpdates;

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
            match producer.make_snapshot() {
                Ok(snapshot) => yield snapshot,
                Err(err) => error!("Failed to produce snapshot: {err}"),
            }

            tokio::select! {
                () = shutdown.cancelled() => break,
                changed = update_rx.changed() => {
                    if changed.is_err() {
                        break;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicI32;
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    use anyhow::Result;
    use futures::StreamExt as _;
    use tokio::sync::watch;
    use tokio::time::timeout;

    use super::*;

    struct CounterProducer {
        update_tx: watch::Sender<()>,
        value: AtomicI32,
    }

    impl CounterProducer {
        fn new() -> Self {
            let (update_tx, _initial_rx) = watch::channel(());

            Self {
                update_tx,
                value: AtomicI32::new(0),
            }
        }

        fn bump(&self) {
            self.value.fetch_add(1, Ordering::AcqRel);
            self.update_tx.send_replace(());
        }
    }

    impl ProducesSnapshot for CounterProducer {
        type Snapshot = i32;

        fn make_snapshot(&self) -> Result<Self::Snapshot> {
            Ok(self.value.load(Ordering::Acquire))
        }
    }

    impl SubscribesToUpdates for CounterProducer {
        fn subscribe_to_updates(&self) -> watch::Receiver<()> {
            self.update_tx.subscribe()
        }
    }

    #[tokio::test]
    async fn snapshots_stream_emits_initial_snapshot() -> Result<()> {
        let producer = Arc::new(CounterProducer::new());
        let shutdown = CancellationToken::new();
        let mut stream = Box::pin(snapshots_stream(producer.clone(), shutdown.clone()));

        let first = timeout(Duration::from_secs(1), stream.next())
            .await
            .map_err(|err| anyhow::anyhow!("initial snapshot did not arrive: {err}"))?
            .ok_or_else(|| anyhow::anyhow!("stream ended before yielding initial snapshot"))?;

        assert_eq!(first, 0);

        Ok(())
    }

    #[tokio::test]
    async fn snapshots_stream_emits_after_subscribed_signal() -> Result<()> {
        let producer = Arc::new(CounterProducer::new());
        let shutdown = CancellationToken::new();
        let mut stream = Box::pin(snapshots_stream(producer.clone(), shutdown.clone()));

        stream
            .next()
            .await
            .ok_or_else(|| anyhow::anyhow!("stream ended before initial snapshot"))?;

        producer.bump();

        let next = timeout(Duration::from_secs(1), stream.next())
            .await
            .map_err(|err| anyhow::anyhow!("snapshot after signal did not arrive: {err}"))?
            .ok_or_else(|| anyhow::anyhow!("stream ended before yielding bumped snapshot"))?;

        assert_eq!(next, 1);

        Ok(())
    }

    #[tokio::test]
    async fn snapshots_stream_terminates_on_shutdown() -> Result<()> {
        let producer = Arc::new(CounterProducer::new());
        let shutdown = CancellationToken::new();
        let mut stream = Box::pin(snapshots_stream(producer.clone(), shutdown.clone()));

        stream
            .next()
            .await
            .ok_or_else(|| anyhow::anyhow!("stream ended before initial snapshot"))?;

        shutdown.cancel();

        let terminated = timeout(Duration::from_secs(1), stream.next())
            .await
            .map_err(|err| anyhow::anyhow!("stream did not close after shutdown: {err}"))?;

        assert!(terminated.is_none());

        Ok(())
    }
}
