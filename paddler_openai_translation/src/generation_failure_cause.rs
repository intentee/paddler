#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenerationFailureCause {
    AgentFailed,
    InvalidRequest,
    Unavailable,
}
