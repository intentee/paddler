use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::agent_controller_pool::AgentControllerPool;
use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use crate::balancer_desired_state_store::BalancerDesiredStateStore;
use crate::buffered_request_manager::BufferedRequestManager;

pub struct AppData {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    pub balancer_desired_state_store: Arc<BalancerDesiredStateStore>,
    pub buffered_request_manager: Arc<BufferedRequestManager>,
    pub shutdown: CancellationToken,
    pub statsd_prefix: String,
}
