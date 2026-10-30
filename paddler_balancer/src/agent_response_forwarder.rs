use std::pin::pin;
use std::sync::Arc;
use std::time::Duration;

use tokio::select;
use tokio::time::Instant;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use tokio_util::sync::DropGuard;

use paddler_messaging::streamable_result::StreamableResult;
use paddler_outbox::outbox::Outbox;

use crate::agent_relay_event::AgentRelayEvent;
use crate::agent_response_forwarding_mode::AgentResponseForwardingMode;
use crate::agent_response_receiver::AgentResponseReceiver;
use crate::agent_stop_follow_up::AgentStopFollowUp;
use crate::dispatched_agent::DispatchedAgent;
use crate::forwarding_event::ForwardingEvent;
use crate::forwarding_plan::ForwardingPlan;

pub struct AgentResponseForwarder<TResponse>
where
    TResponse: StreamableResult,
{
    pub agent_events: Arc<Outbox<AgentRelayEvent<TResponse::Partial, TResponse::Final>>>,
    pub dispatched_agent: DispatchedAgent,
    pub inference_item_timeout: Duration,
    pub relay_abandonment: CancellationToken,
    pub response_receiver: AgentResponseReceiver<TResponse>,
    pub settlement_guard: DropGuard,
    pub shutdown: CancellationToken,
}

impl<TResponse> AgentResponseForwarder<TResponse>
where
    TResponse: StreamableResult,
{
    pub async fn forward(self) {
        let Self {
            agent_events,
            dispatched_agent,
            inference_item_timeout,
            relay_abandonment,
            response_receiver:
                AgentResponseReceiver {
                    request_registration,
                    mut response_rx,
                },
            settlement_guard: _settlement_guard,
            shutdown,
        } = self;
        let agent_controller = &dispatched_agent.agent_controller;
        let agent_connection_close = agent_controller.connection_close.clone();
        let mut forwarding_mode = AgentResponseForwardingMode::ForwardingToRelay;
        let mut item_timeout = pin!(sleep(inference_item_timeout));

        loop {
            item_timeout
                .as_mut()
                .reset(Instant::now() + inference_item_timeout);

            let is_forwarding_to_relay = matches!(
                forwarding_mode,
                AgentResponseForwardingMode::ForwardingToRelay
            );

            let forwarding_event = select! {
                biased;

                () = shutdown.cancelled() => ForwardingEvent::ShutdownRequested,
                () = agent_connection_close.cancelled() => ForwardingEvent::AgentConnectionClosed,
                () = relay_abandonment.cancelled(), if is_forwarding_to_relay => {
                    ForwardingEvent::RelayAbandoned
                }
                () = &mut item_timeout => ForwardingEvent::ItemTimedOut,
                Some(response) = response_rx.recv() => {
                    ForwardingEvent::ResponseReceived(response.into_step())
                }
            };

            let agent_stop_follow_up =
                match forwarding_mode.plan_for(forwarding_event, inference_item_timeout) {
                    ForwardingPlan::Finish => break,
                    ForwardingPlan::IgnoreDrainedResponse => continue,
                    ForwardingPlan::NotifyRelay(agent_relay_event) => {
                        agent_events.push(agent_relay_event);

                        continue;
                    }
                    ForwardingPlan::NotifyRelayThenFinish(agent_relay_event) => {
                        agent_events.push(agent_relay_event);

                        break;
                    }
                    ForwardingPlan::NotifyRelayThenStopAgent(agent_relay_event) => {
                        agent_events.push(agent_relay_event);

                        AgentStopFollowUp::DrainUntilAgentConfirms
                    }
                    ForwardingPlan::NotifyRelayThenStopAgentThenFinish(agent_relay_event) => {
                        agent_events.push(agent_relay_event);

                        AgentStopFollowUp::Finish
                    }
                    ForwardingPlan::StopAgent => AgentStopFollowUp::DrainUntilAgentConfirms,
                };

            agent_controller.stop_responding_to(request_registration.request_id.clone());

            match agent_stop_follow_up {
                AgentStopFollowUp::DrainUntilAgentConfirms => {
                    forwarding_mode = AgentResponseForwardingMode::DrainingUntilAgentConfirms;
                }
                AgentStopFollowUp::Finish => break,
            }
        }
    }
}
