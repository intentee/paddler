use std::future::pending;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use anyhow::anyhow;
use iced::futures::StreamExt as _;
use iced::futures::channel::mpsc;
use paddler::produces_snapshot::ProducesSnapshot;
use paddler::subscribes_to_updates::SubscribesToUpdates;
use paddler_gui::drive_agent_stream_inner::drive_agent_stream_inner;
use paddler_gui::message::Message;
use paddler_types::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;
use tokio::sync::watch;

const AGENT_STREAM_TIMEOUT: Duration = Duration::from_secs(5);

struct FailingSnapshotSource;

impl ProducesSnapshot for FailingSnapshotSource {
    type Snapshot = SlotAggregatedStatusSnapshot;

    fn make_snapshot(&self) -> anyhow::Result<Self::Snapshot> {
        Err(anyhow!("simulated snapshot failure"))
    }
}

impl SubscribesToUpdates for FailingSnapshotSource {
    fn subscribe_to_updates(&self) -> watch::Receiver<()> {
        let (_, rx) = watch::channel(());
        rx
    }
}

struct ImmediatelyDisconnectedUpdateSource;

impl ProducesSnapshot for ImmediatelyDisconnectedUpdateSource {
    type Snapshot = SlotAggregatedStatusSnapshot;

    fn make_snapshot(&self) -> anyhow::Result<Self::Snapshot> {
        Ok(SlotAggregatedStatusSnapshot::default())
    }
}

impl SubscribesToUpdates for ImmediatelyDisconnectedUpdateSource {
    fn subscribe_to_updates(&self) -> watch::Receiver<()> {
        let (_, rx) = watch::channel(());
        rx
    }
}

#[tokio::test]
async fn snapshot_failure_exits_the_agent_stream_before_emitting_any_message() -> Result<()> {
    let (output, mut receiver) = mpsc::channel::<Message>(8);

    let driver = tokio::spawn(drive_agent_stream_inner(
        Arc::new(FailingSnapshotSource),
        pending::<anyhow::Result<()>>(),
        output,
    ));

    tokio::time::timeout(AGENT_STREAM_TIMEOUT, driver).await??;

    assert!(receiver.next().await.is_none());

    Ok(())
}

#[tokio::test]
async fn update_channel_disconnection_exits_the_agent_stream_after_the_first_snapshot()
-> Result<()> {
    let (output, mut receiver) = mpsc::channel::<Message>(8);

    let driver = tokio::spawn(drive_agent_stream_inner(
        Arc::new(ImmediatelyDisconnectedUpdateSource),
        pending::<anyhow::Result<()>>(),
        output,
    ));

    let first_message =
        tokio::time::timeout(AGENT_STREAM_TIMEOUT, receiver.next()).await?;

    assert!(matches!(
        first_message,
        Some(Message::AgentRunning(_))
    ));

    tokio::time::timeout(AGENT_STREAM_TIMEOUT, driver).await??;

    Ok(())
}

struct StaticUpdateSource(watch::Receiver<()>);

impl ProducesSnapshot for StaticUpdateSource {
    type Snapshot = SlotAggregatedStatusSnapshot;

    fn make_snapshot(&self) -> anyhow::Result<Self::Snapshot> {
        Ok(SlotAggregatedStatusSnapshot::default())
    }
}

impl SubscribesToUpdates for StaticUpdateSource {
    fn subscribe_to_updates(&self) -> watch::Receiver<()> {
        self.0.clone()
    }
}

#[tokio::test]
async fn agent_runner_completion_with_error_emits_agent_failed_message() -> Result<()> {
    let (update_tx, update_rx) = watch::channel(());
    let source = Arc::new(StaticUpdateSource(update_rx));
    let (output, mut receiver) = mpsc::channel::<Message>(8);

    let completion = async { Err(anyhow!("agent runner exited unexpectedly")) };

    let driver = tokio::spawn(drive_agent_stream_inner(source, completion, output));

    // Hold the sender alive long enough to ensure the completion future resolves first.
    let collected = async {
        let mut observed_failed = false;
        while let Some(message) = receiver.next().await {
            if matches!(message, Message::AgentFailed(_)) {
                observed_failed = true;
                break;
            }
        }
        observed_failed
    };

    let observed_failed = tokio::time::timeout(AGENT_STREAM_TIMEOUT, collected).await?;
    drop(update_tx);

    tokio::time::timeout(AGENT_STREAM_TIMEOUT, driver).await??;

    assert!(observed_failed);
    Ok(())
}

struct StaticSource;

impl ProducesSnapshot for StaticSource {
    type Snapshot = SlotAggregatedStatusSnapshot;

    fn make_snapshot(&self) -> anyhow::Result<Self::Snapshot> {
        Ok(SlotAggregatedStatusSnapshot::default())
    }
}

impl SubscribesToUpdates for StaticSource {
    fn subscribe_to_updates(&self) -> watch::Receiver<()> {
        let (_, rx) = watch::channel(());
        rx
    }
}

#[tokio::test]
async fn snapshot_send_failure_after_first_iteration_exits_the_agent_stream() -> Result<()> {
    let (output, receiver) = mpsc::channel::<Message>(8);
    drop(receiver);

    let driver = tokio::spawn(drive_agent_stream_inner(
        Arc::new(StaticSource),
        pending::<anyhow::Result<()>>(),
        output,
    ));

    tokio::time::timeout(AGENT_STREAM_TIMEOUT, driver).await??;

    Ok(())
}
