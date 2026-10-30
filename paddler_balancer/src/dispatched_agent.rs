use std::num::NonZeroU16;
use std::sync::Arc;

use crate::agent_controller::AgentController;
use crate::agent_controller_slot_guard::AgentControllerSlotGuard;

pub struct DispatchedAgent {
    pub agent_controller: Arc<AgentController>,
    slot_guard: AgentControllerSlotGuard,
}

impl DispatchedAgent {
    pub const fn new(
        agent_controller: Arc<AgentController>,
        slot_guard: AgentControllerSlotGuard,
    ) -> Self {
        Self {
            agent_controller,
            slot_guard,
        }
    }

    #[must_use]
    pub const fn reserved_slots(&self) -> NonZeroU16 {
        self.slot_guard.reserved_slots()
    }
}
