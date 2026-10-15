use paddler_cli_tests::start_signalled_subprocess_cluster::start_signalled_subprocess_cluster;
use paddler_cli_tests::subprocess_cluster::SubprocessCluster;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

#[tokio::test(flavor = "multi_thread")]
async fn agent_shuts_down_cleanly_while_its_balancer_is_unresponsive() {
    let SubprocessCluster {
        balancer_signals,
        mut cluster,
    } = start_signalled_subprocess_cluster(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        ClusterParams {
            agents: AgentConfig::uniform(1, 1),
            wait_for_slots_ready: false,
            ..ClusterParams::default()
        },
    )
    .await
    .expect("a cluster with a model-less agent must start");

    balancer_signals
        .pause()
        .expect("the balancer must stop responding");

    cluster
        .agents
        .pop()
        .expect("the cluster must have an agent")
        .shutdown()
        .await
        .expect("the agent must shut down cleanly while its balancer does not respond");

    balancer_signals
        .resume()
        .expect("the balancer must continue so that it can shut down");
    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
