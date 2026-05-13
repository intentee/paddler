use std::sync::Arc;

use crate::balancer::agent_controller::AgentController;

pub struct DispatchCandidate {
    pub agent_controller: Arc<AgentController>,
    pub snapshot: i32,
}
