use std::sync::Arc;

use async_stream::stream;
use iced::futures::Stream;
use tokio::pin;
use tokio::select;

use paddler_agent_runner::agent_runner::AgentRunner;
use paddler_agent_runner::agent_runner_params::AgentRunnerParams;
use paddler_messaging::produces_snapshot::ProducesSnapshot as _;
use paddler_messaging::subscribes_to_updates::SubscribesToUpdates as _;

use crate::agent_running_message::AgentRunningMessage;
use crate::message::Message;

pub fn agent_runner_messages(
    agent_runner_params: AgentRunnerParams,
) -> impl Stream<Item = Message> {
    stream! {
        let runner = AgentRunner::start(agent_runner_params);
        let slot_aggregated_status = runner.slot_aggregated_status.clone();
        let mut balancer_connection_rx = runner.balancer_connection_rx.clone();
        let mut status_update_rx = slot_aggregated_status.subscribe_to_updates();
        let completion = runner.wait_for_completion();

        pin!(completion);

        let agent_status_updated = || {
            Message::AgentRunning(AgentRunningMessage::AgentStatusUpdated {
                slots_processing: slot_aggregated_status.slots_processing_count(),
                status: slot_aggregated_status.make_snapshot().status,
            })
        };

        let initial_balancer_connection = *balancer_connection_rx.borrow_and_update();

        yield agent_status_updated();
        yield Message::AgentRunning(AgentRunningMessage::BalancerConnectionChanged(
            initial_balancer_connection,
        ));

        let completion_result = loop {
            let message = select! {
                Ok(()) = status_update_rx.changed() => agent_status_updated(),
                Ok(()) = balancer_connection_rx.changed() => Message::AgentRunning(
                    AgentRunningMessage::BalancerConnectionChanged(
                        *balancer_connection_rx.borrow_and_update(),
                    ),
                ),
                completion_result = &mut completion => break completion_result,
            };

            yield message;
        };

        yield match completion_result {
            Ok(()) => Message::AgentStopped,
            Err(error) => Message::AgentFailed(Arc::new(error)),
        };
    }
}
