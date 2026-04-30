use std::sync::Arc;

use crate::balancer::agent_controller::AgentController;
use crate::balancer::agent_controller_slot_guard::AgentControllerSlotGuard;

pub struct DispatchedAgent {
    pub agent_controller: Arc<AgentController>,
    _slot_guard: AgentControllerSlotGuard,
}

impl DispatchedAgent {
    pub const fn new(
        agent_controller: Arc<AgentController>,
        slot_guard: AgentControllerSlotGuard,
    ) -> Self {
        Self {
            agent_controller,
            _slot_guard: slot_guard,
        }
    }
}
