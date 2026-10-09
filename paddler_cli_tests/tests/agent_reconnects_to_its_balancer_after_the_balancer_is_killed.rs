use paddler_cli_tests::restart_balancer::restart_balancer;
use paddler_cli_tests::start_signalled_subprocess_cluster::start_signalled_subprocess_cluster;
use paddler_cli_tests::subprocess_cluster::SubprocessCluster;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

#[tokio::test(flavor = "multi_thread")]
async fn agent_reconnects_to_its_balancer_after_the_balancer_is_killed() {
    let SubprocessCluster {
        balancer_signals,
        cluster,
    } = start_signalled_subprocess_cluster(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        ClusterParams {
            agents: vec![AgentConfig::single(1)],
            wait_for_slots_ready: false,
            ..ClusterParams::default()
        },
    )
    .await
    .expect("a cluster with a model-less agent must start");

    balancer_signals
        .kill()
        .expect("the balancer must be killed without closing its connections");

    let mut restarted_cluster =
        restart_balancer(env!("CARGO_BIN_EXE_paddler_cluster_node"), cluster)
            .await
            .expect("the balancer must restart on its management address");

    restarted_cluster
        .wait_for_agent_count(1)
        .await
        .expect("the agent must reconnect to the restarted balancer");

    restarted_cluster
        .shutdown()
        .await
        .expect("the restarted cluster must shut down cleanly");
}
