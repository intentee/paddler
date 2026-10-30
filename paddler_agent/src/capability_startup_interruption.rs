use crate::agent_error::AgentError;

#[derive(Debug)]
pub enum CapabilityStartupInterruption {
    Cancelled,
    Failed(AgentError),
}
