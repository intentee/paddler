use std::sync::Arc;
use std::time::Duration;

use actix_web_lab::sse;
use futures::Stream;
use futures::StreamExt as _;
use paddler_messaging::produces_snapshot::ProducesSnapshot;
use paddler_messaging::subscribes_to_updates::SubscribesToUpdates;
use tokio_util::sync::CancellationToken;

use crate::snapshots_stream::snapshots_stream;

const KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(10);

pub fn sse_response_from_snapshots<TProducer>(
    producer: Arc<TProducer>,
    shutdown: CancellationToken,
) -> sse::Sse<impl Stream<Item = Result<sse::Event, serde_json::Error>>>
where
    TProducer: ProducesSnapshot + SubscribesToUpdates + Send + Sync + 'static,
    TProducer::Snapshot: Send + 'static,
{
    sse::Sse::from_stream(
        snapshots_stream(producer, shutdown)
            .map(|snapshot| sse::Data::new_json(snapshot).map(sse::Event::Data)),
    )
    .with_keep_alive(KEEP_ALIVE_INTERVAL)
}
