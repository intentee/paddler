use crate::registered_agent_controller_guard::RegisteredAgentControllerGuard;

pub enum AgentControllerRegistration {
    DuplicateAgentId,
    Registered(RegisteredAgentControllerGuard),
}
