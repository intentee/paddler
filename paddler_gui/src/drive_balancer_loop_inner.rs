use std::future::Future;

use anyhow::Result;
use iced::futures::SinkExt as _;
use iced::futures::channel::mpsc::Sender;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use tokio::sync::broadcast;
use tokio::sync::watch;

use crate::message::Message;
use crate::running_balancer_handler;
use crate::running_balancer_snapshot::RunningBalancerSnapshot;

pub async fn drive_balancer_loop_inner<TSnapshotFn, TCompletion>(
    initial_desired_state: BalancerDesiredState,
    mut pool_update_rx: watch::Receiver<()>,
    mut holder_update_rx: watch::Receiver<()>,
    mut desired_state_rx: broadcast::Receiver<BalancerDesiredState>,
    completion_future: TCompletion,
    mut snapshot_fn: TSnapshotFn,
    mut output: Sender<Message>,
) where
    TSnapshotFn: FnMut(&BalancerDesiredState) -> Result<RunningBalancerSnapshot> + Send,
    TCompletion: Future<Output = Result<()>> + Send,
{
    let mut current_desired_state = initial_desired_state;
    tokio::pin!(completion_future);

    loop {
        match snapshot_fn(&current_desired_state) {
            Ok(snapshot) => {
                if output
                    .send(Message::RunningBalancer(
                        running_balancer_handler::Message::SnapshotUpdated(Box::new(snapshot)),
                    ))
                    .await
                    .is_err()
                {
                    return;
                }
            }
            Err(error) => {
                log::error!("Failed to build running balancer snapshot: {error}");

                return;
            }
        }

        tokio::select! {
            changed = pool_update_rx.changed() => {
                if changed.is_err() {
                    return;
                }
            }
            changed = holder_update_rx.changed() => {
                if changed.is_err() {
                    return;
                }
            }
            desired_state_result = desired_state_rx.recv() => {
                match desired_state_result {
                    Ok(new_desired_state) => {
                        current_desired_state = new_desired_state;
                    }
                    Err(broadcast::error::RecvError::Lagged(missed)) => {
                        log::warn!(
                            "Desired-state broadcast lagged by {missed} messages; \
                             continuing with the last known state"
                        );
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        log::info!(
                            "Desired-state broadcast closed; ending snapshot stream"
                        );

                        return;
                    }
                }
            }
            result = &mut completion_future => {
                match result {
                    Ok(()) => {
                        if let Err(err) = output.send(Message::BalancerStopped).await {
                            log::warn!(
                                "Failed to deliver BalancerStopped to UI (receiver dropped): {err}"
                            );
                        }
                    }
                    Err(error) => {
                        let detail = error.to_string();
                        if let Err(err) = output
                            .send(Message::BalancerFailed(detail.clone()))
                            .await
                        {
                            log::error!(
                                "Failed to deliver BalancerFailed to UI (receiver dropped); lost detail: {detail}; send err: {err}"
                            );
                        }
                    }
                }

                return;
            }
        }
    }
}
