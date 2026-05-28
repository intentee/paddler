use paddler_client::PaddlerClient;
use tokio::process::Child;
use tokio_util::sync::CancellationToken;

use crate::agents_stream_watcher::AgentsStreamWatcher;
use crate::balancer_addresses::BalancerAddresses;
use crate::buffered_requests_stream_watcher::BufferedRequestsStreamWatcher;

pub struct ClusterHandleParams {
    pub addresses: BalancerAddresses,
    pub agent_ids: Vec<String>,
    pub agent_subprocesses: Vec<Child>,
    pub agents: AgentsStreamWatcher,
    pub balancer_subprocess: Child,
    pub buffered_requests: BufferedRequestsStreamWatcher,
    pub cancel_token: CancellationToken,
    pub paddler_client: PaddlerClient,
}
