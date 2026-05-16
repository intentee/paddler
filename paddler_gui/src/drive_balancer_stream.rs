use iced::futures::SinkExt as _;
use iced::futures::channel::mpsc::Sender;
use paddler::subscribes_to_updates::SubscribesToUpdates as _;
use paddler_bootstrap::balancer_runner::BalancerRunner;
use paddler_bootstrap::balancer_runner::BalancerRunnerParams;
use tokio::sync::broadcast;

use crate::message::Message;
use crate::running_balancer_handler;
use crate::running_balancer_snapshot::RunningBalancerSnapshot;

pub async fn drive_balancer_stream(params: BalancerRunnerParams, mut output: Sender<Message>) {
    let mut runner = match BalancerRunner::start(params).await {
        Ok(runner) => runner,
        Err(error) => {
            let detail = error.to_string();
            if let Err(err) = output.send(Message::BalancerFailed(detail.clone())).await {
                log::error!(
                    "Failed to deliver BalancerFailed to UI (receiver dropped); lost detail: {detail}; send err: {err}"
                );
            }

            return;
        }
    };

    let completion_future = runner.wait_for_completion();
    tokio::pin!(completion_future);

    if output.send(Message::BalancerStarted).await.is_err() {
        return;
    }

    let mut desired_state_rx = runner.balancer_desired_state_tx.subscribe();
    let mut current_desired_state = runner.initial_desired_state.clone();
    let mut pool_update_rx = runner.agent_controller_pool.subscribe_to_updates();
    let mut holder_update_rx = runner
        .balancer_applicable_state_holder
        .subscribe_to_updates();

    loop {
        match RunningBalancerSnapshot::build(
            &runner.agent_controller_pool,
            &runner.balancer_applicable_state_holder,
            current_desired_state.clone(),
        ) {
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
