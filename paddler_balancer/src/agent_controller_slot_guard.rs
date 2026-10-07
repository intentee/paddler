use std::sync::Arc;

use crate::agent_controller::AgentController;
use crate::agent_controller_pool_updates::AgentControllerPoolUpdates;

pub struct AgentControllerSlotGuard {
    agent_controller: Arc<AgentController>,
    pool_updates: Arc<AgentControllerPoolUpdates>,
}

impl AgentControllerSlotGuard {
    pub const fn new(
        agent_controller: Arc<AgentController>,
        pool_updates: Arc<AgentControllerPoolUpdates>,
    ) -> Self {
        Self {
            agent_controller,
            pool_updates,
        }
    }
}

impl Drop for AgentControllerSlotGuard {
    fn drop(&mut self) {
        self.agent_controller.slots_processing.decrement();
        self.pool_updates.signal();
    }
}
