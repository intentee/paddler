use std::sync::Arc;

use log::info;

use crate::agent_controller_pool::AgentControllerPool;

pub struct RegisteredAgentControllerGuard {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub agent_id: String,
}

impl Drop for RegisteredAgentControllerGuard {
    fn drop(&mut self) {
        self.agent_controller_pool
            .remove_agent_controller(&self.agent_id);

        info!("Removed agent: {}", self.agent_id);
    }
}
