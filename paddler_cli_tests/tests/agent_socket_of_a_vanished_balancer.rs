use paddler_cli_tests::start_signalled_subprocess_cluster::start_signalled_subprocess_cluster;
use paddler_cli_tests::subprocess_cluster::SubprocessCluster;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::untrusted_agent_socket_client::UntrustedAgentSocketClient;

const AGENT_ID_OF_A_VANISHED_BALANCER: &str = "agent-of-a-vanished-balancer";

pub struct AgentSocketOfAVanishedBalancer {
    pub agent_socket: UntrustedAgentSocketClient,
    pub cluster: Cluster,
}

impl AgentSocketOfAVanishedBalancer {
    pub async fn connect() -> Self {
        let SubprocessCluster {
            balancer_signals,
            cluster,
        } = start_signalled_subprocess_cluster(
            env!("CARGO_BIN_EXE_paddler_cluster_node"),
            ClusterParams::without_agents(),
        )
        .await
        .expect("a cluster without agents must start");
        let mut agent_socket = UntrustedAgentSocketClient::connect(
            cluster.balancer.addresses.management,
            AGENT_ID_OF_A_VANISHED_BALANCER,
        )
        .await
        .expect("the agent socket must reach the balancer");

        agent_socket
            .next_text()
            .await
            .expect("the balancer must announce its version before it is killed");
        balancer_signals
            .kill()
            .expect("the balancer must be killed");

        Self {
            agent_socket,
            cluster,
        }
    }

    pub async fn shut_down(self) {
        self.cluster
            .shutdown()
            .await
            .expect("the cluster must shut down after its balancer was killed");
    }
}
