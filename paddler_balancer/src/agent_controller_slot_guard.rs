use std::num::NonZeroU16;
use std::sync::Arc;

use crate::agent_controller::AgentController;
use crate::agent_controller_pool_updates::AgentControllerPoolUpdates;

pub struct AgentControllerSlotGuard {
    agent_controller: Arc<AgentController>,
    pool_updates: Arc<AgentControllerPoolUpdates>,
    reserved_slots: NonZeroU16,
}

impl AgentControllerSlotGuard {
    pub const fn new(
        agent_controller: Arc<AgentController>,
        pool_updates: Arc<AgentControllerPoolUpdates>,
        reserved_slots: NonZeroU16,
    ) -> Self {
        Self {
            agent_controller,
            pool_updates,
            reserved_slots,
        }
    }

    #[must_use]
    pub const fn reserved_slots(&self) -> NonZeroU16 {
        self.reserved_slots
    }
}

impl Drop for AgentControllerSlotGuard {
    fn drop(&mut self) {
        self.agent_controller
            .slots_processing
            .subtract(u64::from(self.reserved_slots.get()));
        self.pool_updates.signal();
    }
}
