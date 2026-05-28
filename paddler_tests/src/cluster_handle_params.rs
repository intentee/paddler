use paddler_bootstrap::agent_runner::AgentRunner;
use paddler_bootstrap::balancer_runner::BalancerRunner;
use paddler_client::PaddlerClient;
use tokio_util::sync::CancellationToken;

use crate::agents_stream_watcher::AgentsStreamWatcher;
use crate::balancer_addresses::BalancerAddresses;
use crate::buffered_requests_stream_watcher::BufferedRequestsStreamWatcher;

pub struct ClusterHandleParams {
    pub addresses: BalancerAddresses,
    pub agent_ids: Vec<String>,
    pub agent_runners: Vec<AgentRunner>,
    pub agents: AgentsStreamWatcher,
    pub balancer_runner: BalancerRunner,
    pub buffered_requests: BufferedRequestsStreamWatcher,
    pub cancel_token: CancellationToken,
    pub paddler_client: PaddlerClient,
}
