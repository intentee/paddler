use std::sync::Arc;

use tokio::sync::watch;

use crate::balancer::agent_controller::AgentController;

pub struct AgentControllerSlotGuard {
    agent_controller: Arc<AgentController>,
    pool_update_tx: watch::Sender<()>,
}

impl AgentControllerSlotGuard {
    pub const fn new(
        agent_controller: Arc<AgentController>,
        pool_update_tx: watch::Sender<()>,
    ) -> Self {
        Self {
            agent_controller,
            pool_update_tx,
        }
    }
}

impl Drop for AgentControllerSlotGuard {
    fn drop(&mut self) {
        self.agent_controller.slots_processing.decrement();
        self.pool_update_tx.send_replace(());
    }
}
