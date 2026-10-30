use crate::relay_failure::RelayFailure;

#[derive(Debug, Eq, PartialEq)]
pub enum AgentRelayEvent<TPartial, TFinal> {
    Failed(RelayFailure),
    Final(TFinal),
    Partial(TPartial),
}
