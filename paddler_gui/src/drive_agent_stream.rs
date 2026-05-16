use iced::futures::SinkExt as _;
use iced::futures::channel::mpsc::Sender;
use paddler::produces_snapshot::ProducesSnapshot as _;
use paddler::subscribes_to_updates::SubscribesToUpdates as _;
use paddler_bootstrap::agent_runner::AgentRunner;
use paddler_bootstrap::agent_runner::AgentRunnerParams;

use crate::agent_running_handler;
use crate::message::Message;

pub async fn drive_agent_stream(params: AgentRunnerParams, mut output: Sender<Message>) {
    let mut runner = AgentRunner::start(params);

    let slot_aggregated_status = runner.slot_aggregated_status.clone();
    let mut update_rx = slot_aggregated_status.subscribe_to_updates();
    let completion_future = runner.wait_for_completion();
    tokio::pin!(completion_future);

    loop {
        match slot_aggregated_status.make_snapshot() {
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
