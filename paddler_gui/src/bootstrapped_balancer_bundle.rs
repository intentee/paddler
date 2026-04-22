use std::sync::Arc;

use paddler::balancer::agent_controller_pool::AgentControllerPool;
use paddler::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use tokio::sync::broadcast;

pub struct BootstrappedBalancerBundle {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    pub balancer_desired_state_rx: broadcast::Receiver<BalancerDesiredState>,
    pub initial_desired_state: BalancerDesiredState,
}
