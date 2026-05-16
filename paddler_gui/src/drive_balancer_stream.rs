use iced::futures::SinkExt as _;
use iced::futures::channel::mpsc::Sender;
use paddler::subscribes_to_updates::SubscribesToUpdates as _;
use paddler_bootstrap::balancer_runner::BalancerRunner;
use paddler_bootstrap::balancer_runner::BalancerRunnerParams;
use paddler_types::balancer_desired_state::BalancerDesiredState;

use crate::drive_balancer_loop_inner::drive_balancer_loop_inner;
use crate::message::Message;
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

    if output.send(Message::BalancerStarted).await.is_err() {
        return;
    }

    let desired_state_rx = runner.balancer_desired_state_tx.subscribe();
    let initial_desired_state = runner.initial_desired_state.clone();
    let pool_update_rx = runner.agent_controller_pool.subscribe_to_updates();
    let holder_update_rx = runner
        .balancer_applicable_state_holder
        .subscribe_to_updates();

    let pool = runner.agent_controller_pool.clone();
    let holder = runner.balancer_applicable_state_holder.clone();

    let snapshot_fn = move |state: &BalancerDesiredState| {
        RunningBalancerSnapshot::build(&pool, &holder, state.clone())
    };

    drive_balancer_loop_inner(
        initial_desired_state,
        pool_update_rx,
        holder_update_rx,
        desired_state_rx,
        completion_future,
        snapshot_fn,
        output,
    )
    .await;
}
