use paddler_client::PaddlerClient;
use tokio_util::sync::CancellationToken;

use crate::agents_stream_watcher::AgentsStreamWatcher;
use crate::balancer_addresses::BalancerAddresses;
use crate::buffered_requests_stream_watcher::BufferedRequestsStreamWatcher;
use crate::cluster_completion::ClusterCompletion;

pub struct ClusterHandleParams {
    pub addresses: BalancerAddresses,
    pub agent_ids: Vec<String>,
    pub agents: AgentsStreamWatcher,
    pub buffered_requests: BufferedRequestsStreamWatcher,
    pub cancel_token: CancellationToken,
    pub completion: ClusterCompletion,
    pub paddler_client: PaddlerClient,
}
