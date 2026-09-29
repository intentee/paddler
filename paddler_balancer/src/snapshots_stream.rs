use std::sync::Arc;

use async_stream::stream;
use futures::Stream;
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
    use std::sync::Arc;
    use std::sync::atomic::AtomicI32;
    use std::sync::atomic::Ordering;

    use futures::StreamExt as _;
    use paddler_messaging::produces_snapshot::ProducesSnapshot;
    use paddler_messaging::subscribes_to_updates::SubscribesToUpdates;
    use tokio::sync::watch;
    use tokio_util::sync::CancellationToken;

    use super::snapshots_stream;

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

        fn make_snapshot(&self) -> Self::Snapshot {
            self.value.load(Ordering::Acquire)
        }
    }

    impl SubscribesToUpdates for CounterProducer {
        fn subscribe_to_updates(&self) -> watch::Receiver<()> {
            self.update_tx.subscribe()
        }
    }

    #[tokio::test]
    async fn snapshots_stream_emits_after_subscribed_signal() {
        let producer = Arc::new(CounterProducer::new());
        let shutdown = CancellationToken::new();
        let mut stream = Box::pin(snapshots_stream(producer.clone(), shutdown.clone()));

        assert_eq!(stream.next().await, Some(0));

        producer.bump();

        assert_eq!(stream.next().await, Some(1));
    }

    #[tokio::test]
    async fn snapshots_stream_terminates_on_shutdown() {
        let producer = Arc::new(CounterProducer::new());
        let shutdown = CancellationToken::new();
        let mut stream = Box::pin(snapshots_stream(producer.clone(), shutdown.clone()));

        assert_eq!(stream.next().await, Some(0));

        shutdown.cancel();

        assert_eq!(stream.next().await, None);
    }
}
