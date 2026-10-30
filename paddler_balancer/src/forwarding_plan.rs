use crate::agent_relay_event::AgentRelayEvent;

#[derive(Debug, Eq, PartialEq)]
pub enum ForwardingPlan<TPartial, TFinal> {
    Finish,
    IgnoreDrainedResponse,
    NotifyRelay(AgentRelayEvent<TPartial, TFinal>),
    NotifyRelayThenFinish(AgentRelayEvent<TPartial, TFinal>),
    NotifyRelayThenStopAgent(AgentRelayEvent<TPartial, TFinal>),
    NotifyRelayThenStopAgentThenFinish(AgentRelayEvent<TPartial, TFinal>),
    StopAgent,
}
