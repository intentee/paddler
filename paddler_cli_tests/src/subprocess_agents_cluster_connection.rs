use anyhow::Result;
use tokio_util::sync::CancellationToken;

use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::running_balancer::RunningBalancer;

use crate::subprocess_agent_spawner::SubprocessAgentSpawner;

pub struct SubprocessAgentsClusterConnection<'connection> {
    pub agent_binary_path: &'connection str,
    pub cancellation_token: CancellationToken,
    pub desired_state: &'connection ClusterDesiredState,
    pub running_balancer: RunningBalancer,
}

impl SubprocessAgentsClusterConnection<'_> {
    pub async fn connect(self) -> Result<Cluster> {
        let management_addr = self.running_balancer.addresses.management;

        Cluster::connect(
            self.cancellation_token,
            self.running_balancer,
            Box::new(SubprocessAgentSpawner::new(
                self.agent_binary_path.to_owned(),
                management_addr,
            )),
            self.desired_state,
        )
        .await
    }
}
