use crate::dispatched_agent::DispatchedAgent;

pub enum DispatchClaim {
    Claimed(DispatchedAgent),
    Contended,
}
