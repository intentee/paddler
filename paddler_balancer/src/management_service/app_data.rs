use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use paddler_state_database::state_database::StateDatabase;

use crate::agent_controller_pool::AgentControllerPool;
use crate::agent_response_senders::AgentResponseSenders;
use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use crate::buffered_request_manager::BufferedRequestManager;

pub struct AppData {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    pub buffered_request_manager: Arc<BufferedRequestManager>,
    pub agent_response_senders: AgentResponseSenders,
    pub shutdown: CancellationToken,
    pub state_database: Arc<dyn StateDatabase>,
    pub statsd_prefix: String,
}
