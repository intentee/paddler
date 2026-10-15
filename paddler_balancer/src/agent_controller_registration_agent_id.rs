use crate::agent_controller_registration::AgentControllerRegistration;
use crate::registered_agent_controller_guard::RegisteredAgentControllerGuard;

impl AgentControllerRegistration {
    #[must_use]
    pub fn registered_agent_id(&self) -> Option<&str> {
        match self {
            Self::DuplicateAgentId => None,
            Self::Registered(RegisteredAgentControllerGuard {
                agent_controller, ..
            }) => Some(&agent_controller.id),
        }
    }
}
