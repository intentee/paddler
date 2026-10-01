use std::sync::Arc;

use log::info;

use crate::agent_controller::AgentController;
use crate::agent_controller_pool::AgentControllerPool;

pub struct RegisteredAgentControllerGuard {
    pub agent_controller: Arc<AgentController>,
    pub agent_controller_pool: Arc<AgentControllerPool>,
}

impl Drop for RegisteredAgentControllerGuard {
    fn drop(&mut self) {
        self.agent_controller_pool
            .remove_agent_controller(&self.agent_controller.id);

        info!("Removed agent: {}", self.agent_controller.id);
    }
}
