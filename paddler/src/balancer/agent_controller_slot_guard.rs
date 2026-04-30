use std::sync::Arc;

use crate::balancer::agent_controller::AgentController;

pub struct AgentControllerSlotGuard {
    agent_controller: Arc<AgentController>,
}

impl AgentControllerSlotGuard {
    pub const fn new(agent_controller: Arc<AgentController>) -> Self {
        Self { agent_controller }
    }
}

impl Drop for AgentControllerSlotGuard {
    fn drop(&mut self) {
        self.agent_controller.slots_processing.decrement();
    }
}
