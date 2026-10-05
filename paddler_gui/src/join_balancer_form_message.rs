#[derive(Debug, Clone)]
pub enum JoinBalancerFormMessage {
    SetAgentName(String),
    SetBalancerAddress(String),
    SetSlotsCount(String),
    Connect,
    Cancel,
}
