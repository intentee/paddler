use paddler_messaging::result_step::ResultStep;

pub enum ForwardingEvent<TPartial, TFinal> {
    AgentConnectionClosed,
    ItemTimedOut,
    RelayAbandoned,
    ResponseReceived(ResultStep<TPartial, TFinal>),
    ShutdownRequested,
}
