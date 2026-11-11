use std::net::SocketAddr;

use anyhow::Error;
use anyhow::Result;
use futures_util::TryFutureExt as _;

use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::untrusted_agent_socket_client::UntrustedAgentSocketClient;

use crate::start_signalled_subprocess_cluster::start_signalled_subprocess_cluster;
use crate::subprocess_cluster::SubprocessCluster;

const AGENT_ID_OF_A_VANISHED_BALANCER: &str = "agent-of-a-vanished-balancer";

async fn announced_agent_socket(management_addr: SocketAddr) -> Result<UntrustedAgentSocketClient> {
    UntrustedAgentSocketClient::connect(management_addr, AGENT_ID_OF_A_VANISHED_BALANCER)
        .err_into::<Error>()
        .and_then(|mut agent_socket| async move {
            agent_socket
                .next_text()
                .await
                .map(|_announced_version| agent_socket)
                .map_err(Error::from)
        })
        .await
}

pub struct AgentSocketOfAVanishedBalancer {
    pub agent_socket: UntrustedAgentSocketClient,
    pub cluster: Cluster,
}

impl AgentSocketOfAVanishedBalancer {
    /// # Errors
    ///
    /// Fails when a cluster without agents does not start, when the agent socket cannot reach the balancer or is not told its version, and when the balancer cannot be killed.
    pub async fn connect(binary_path: &str) -> Result<Self> {
        start_signalled_subprocess_cluster(binary_path, ClusterParams::without_agents())
            .and_then(Self::killing_the_balancer_of)
            .await
    }

    async fn killing_the_balancer_of(
        SubprocessCluster {
            balancer_signals,
            cluster,
        }: SubprocessCluster,
    ) -> Result<Self> {
        announced_agent_socket(cluster.balancer.addresses.management)
            .await
            .and_then(|agent_socket| {
                balancer_signals
                    .kill()
                    .map(|()| Self {
                        agent_socket,
                        cluster,
                    })
                    .map_err(Error::from)
            })
    }

    /// # Errors
    ///
    /// Fails when an agent or the balancer of the cluster cannot be terminated, or ends with a failure.
    pub async fn shut_down(self) -> Result<()> {
        self.cluster.shutdown().await
    }
}
