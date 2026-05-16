use std::future::Future;
use std::sync::Arc;

use anyhow::Result;
use iced::futures::SinkExt as _;
use iced::futures::channel::mpsc::Sender;
use paddler::produces_snapshot::ProducesSnapshot;
use paddler::subscribes_to_updates::SubscribesToUpdates;
use paddler_types::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

use crate::agent_running_handler;
use crate::message::Message;

pub async fn drive_agent_stream_inner<TSource, TCompletion>(
    snapshot_source: Arc<TSource>,
    completion_future: TCompletion,
    mut output: Sender<Message>,
) where
    TSource: ProducesSnapshot<Snapshot = SlotAggregatedStatusSnapshot>
        + SubscribesToUpdates
        + Send
        + Sync
        + 'static,
    TCompletion: Future<Output = Result<()>> + Send,
{
    let mut update_rx = snapshot_source.subscribe_to_updates();
    tokio::pin!(completion_future);

    loop {
        match snapshot_source.make_snapshot() {
            Ok(snapshot) => {
                if output
                    .send(Message::AgentRunning(
                        agent_running_handler::Message::AgentStatusUpdated(snapshot),
                    ))
                    .await
                    .is_err()
                {
                    return;
                }
            }
            Err(error) => {
                log::error!("Failed to make agent status snapshot: {error}");

                return;
            }
        }

        tokio::select! {
            changed = update_rx.changed() => {
                if changed.is_err() {
                    return;
                }
            }
            result = &mut completion_future => {
                match result {
                    Ok(()) => {
                        if let Err(err) = output.send(Message::AgentStopped).await {
                            log::warn!(
                                "Failed to deliver AgentStopped to UI (receiver dropped): {err}"
                            );
                        }
                    }
                    Err(error) => {
                        let detail = error.to_string();
                        if let Err(err) = output
                            .send(Message::AgentFailed(detail.clone()))
                            .await
                        {
                            log::error!(
                                "Failed to deliver AgentFailed to UI (receiver dropped); lost detail: {detail}; send err: {err}"
                            );
                        }
                    }
                }

                return;
            }
        }
    }
}
