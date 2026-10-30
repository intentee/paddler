use std::sync::Arc;
use std::time::Duration;

use tokio::spawn;
use tokio_util::sync::CancellationToken;
use tokio_util::sync::DropGuard;

use paddler_messaging::streamable_result::StreamableResult;
use paddler_outbox::outbox::Outbox;

use crate::agent_relay_event::AgentRelayEvent;
use crate::agent_response_forwarder::AgentResponseForwarder;
use crate::agent_streaming_request::AgentStreamingRequest;
use crate::dispatched_agent::DispatchedAgent;

pub struct DispatchedAgentRelay<TResponse>
where
    TResponse: StreamableResult,
{
    pub agent_events: Arc<Outbox<AgentRelayEvent<TResponse::Partial, TResponse::Final>>>,
    pub generated_by: Option<String>,
    _relay_abandonment: DropGuard,
}

impl<TResponse> DispatchedAgentRelay<TResponse>
where
    TResponse: StreamableResult<Final: Send + 'static, Partial: Send + 'static> + Send + 'static,
{
    pub fn start<TParams>(
        dispatched_agent: DispatchedAgent,
        params: TParams,
        inference_item_timeout: Duration,
        settlement: CancellationToken,
        shutdown: CancellationToken,
    ) -> Self
    where
        TParams: AgentStreamingRequest<Response = TResponse>,
    {
        let response_receiver = dispatched_agent
            .agent_controller
            .stream_responses_to(params, dispatched_agent.reserved_slots());
        let agent_events = Arc::new(Outbox::default());
        let generated_by = dispatched_agent.agent_controller.name.clone();
        let relay_abandonment = CancellationToken::new();

        spawn(
            AgentResponseForwarder {
                agent_events: agent_events.clone(),
                dispatched_agent,
                inference_item_timeout,
                relay_abandonment: relay_abandonment.clone(),
                response_receiver,
                settlement_guard: settlement.drop_guard(),
                shutdown,
            }
            .forward(),
        );

        Self {
            agent_events,
            generated_by,
            _relay_abandonment: relay_abandonment.drop_guard(),
        }
    }
}
