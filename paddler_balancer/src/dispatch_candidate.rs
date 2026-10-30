use std::num::NonZeroU16;
use std::sync::Arc;

use crate::agent_controller::AgentController;

pub struct DispatchCandidate {
    pub agent_controller: Arc<AgentController>,
    pub reserved_slots: NonZeroU16,
    pub snapshot: u64,
}
