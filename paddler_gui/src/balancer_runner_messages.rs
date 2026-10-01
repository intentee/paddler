use std::sync::Arc;

use async_stream::stream;
use iced::futures::Stream;
use tokio::pin;
use tokio::select;
use tokio::sync::watch;

use paddler_bootstrap::balancer_runner::BalancerRunner;
use paddler_bootstrap::balancer_runner_params::BalancerRunnerParams;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::subscribes_to_updates::SubscribesToUpdates as _;

use crate::message::Message;
use crate::running_balancer_message::RunningBalancerMessage;
use crate::running_balancer_snapshot::RunningBalancerSnapshot;

pub fn balancer_runner_messages(
    balancer_runner_params: BalancerRunnerParams,
) -> impl Stream<Item = Message> {
    stream! {
        let cancellation_token = balancer_runner_params.cancellation_token.clone();
        let runner = match BalancerRunner::start(balancer_runner_params).await {
            Ok(runner) => runner,
            Err(error) => {
                yield Message::BalancerFailed(Arc::new(error));

                return;
            }
        };
        let addresses = runner.addresses;
        let agent_controller_pool = runner.agent_controller_pool.clone();
        let balancer_applicable_state_holder = runner.balancer_applicable_state_holder.clone();
        let mut desired_state_rx = runner.balancer_desired_state_tx.subscribe();
        let mut pool_update_rx = agent_controller_pool.subscribe_to_updates();
        let mut applicable_state_update_rx = balancer_applicable_state_holder.subscribe_to_updates();
        let completion = runner.wait_for_completion();

        pin!(completion);

        let current_snapshot = |desired_state_rx: &mut watch::Receiver<BalancerDesiredState>| {
            Box::new(RunningBalancerSnapshot::build(
                &agent_controller_pool,
                &balancer_applicable_state_holder,
                desired_state_rx.borrow_and_update().clone(),
            ))
        };

        yield Message::BalancerStarted {
            addresses,
            cancellation_token,
            snapshot: current_snapshot(&mut desired_state_rx),
        };

        let completion_result = loop {
            select! {
                Ok(()) = pool_update_rx.changed() => {}
                Ok(()) = applicable_state_update_rx.changed() => {}
                Ok(()) = desired_state_rx.changed() => {}
                completion_result = &mut completion => break completion_result,
            }

            yield Message::RunningBalancer(RunningBalancerMessage::SnapshotUpdated(
                current_snapshot(&mut desired_state_rx),
            ));
        };

        yield match completion_result {
            Ok(()) => Message::BalancerStopped,
            Err(error) => Message::BalancerFailed(Arc::new(error)),
        };
    }
}
