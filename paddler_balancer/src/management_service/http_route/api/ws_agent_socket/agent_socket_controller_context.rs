use std::sync::Arc;

use parking_lot::RwLock;

use super::agent_socket_registration::AgentSocketRegistration;
use crate::agent_controller_pool::AgentControllerPool;
use crate::agent_response_senders::AgentResponseSenders;
use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;

pub struct AgentSocketControllerContext {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub agent_id: String,
    pub agent_response_senders: AgentResponseSenders,
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    pub registration: RwLock<AgentSocketRegistration>,
}
