use paddler_cli_tests::start_subprocess_cluster::start_subprocess_cluster;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_registers_multiple_agents_over_time() {
    let mut cluster = start_subprocess_cluster(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        ClusterParams {
            agents: Vec::new(),
            wait_for_slots_ready: false,
            ..ClusterParams::default()
        },
    )
    .await
    .expect("the cluster must start");

    cluster
        .spawn_additional_agent(&AgentConfig {
            name: "test-agent-1".to_owned(),
            slot_count: 1,
        })
        .expect("the additional agent must start");

    cluster
        .wait_for_agent_count(1)
        .await
        .expect("first agent should register");

    cluster
        .spawn_additional_agent(&AgentConfig {
            name: "test-agent-2".to_owned(),
            slot_count: 1,
        })
        .expect("the additional agent must start");

    cluster
        .wait_for_agent_count(2)
        .await
        .expect("second agent should register");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
