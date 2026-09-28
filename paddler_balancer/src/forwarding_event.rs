pub enum ForwardingEvent<TResponse> {
    AgentConnectionClosed,
    ClientConnectionClosed,
    ItemTimedOut,
    ResponseReceived(TResponse),
    ShutdownRequested,
}
