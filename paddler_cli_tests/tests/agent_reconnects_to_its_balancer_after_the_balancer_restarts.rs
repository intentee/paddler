use tokio_util::sync::CancellationToken;

use paddler_cli_tests::spawn_balancer_subprocess::spawn_balancer_subprocess;
use paddler_cli_tests::spawned_balancer_subprocess::SpawnedBalancerSubprocess;
use paddler_cli_tests::start_subprocess_cluster::start_subprocess_cluster;
use paddler_cli_tests::subprocess_agent_spawner::SubprocessAgentSpawner;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::ephemeral_loopback_addr::EPHEMERAL_LOOPBACK_ADDR;

#[tokio::test(flavor = "multi_thread")]
async fn agent_reconnects_to_its_balancer_after_the_balancer_restarts() {
    let Cluster {
        agents, balancer, ..
    } = start_subprocess_cluster(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        ClusterParams {
            agents: vec![AgentConfig::single(1)],
            wait_for_slots_ready: false,
            ..ClusterParams::default()
        },
    )
    .await
    .expect("a cluster with a model-less agent must start");
    let management_addr = balancer.addresses.management;

    balancer
        .shutdown()
        .await
        .expect("the first balancer must shut down cleanly");

    let SpawnedBalancerSubprocess {
        running_balancer, ..
    } = spawn_balancer_subprocess(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        [
            "--inference-addr".to_owned(),
            EPHEMERAL_LOOPBACK_ADDR.to_string(),
            "--management-addr".to_owned(),
            management_addr.to_string(),
        ],
    )
    .await
    .expect("the restarted balancer must take over the management address");
    let mut restarted_cluster = Cluster::connect(
        CancellationToken::new(),
        running_balancer,
        Box::new(SubprocessAgentSpawner::new(
            env!("CARGO_BIN_EXE_paddler_cluster_node").to_owned(),
            management_addr,
        )),
        &ClusterDesiredState::KeepStored,
    )
    .await
    .expect("the restarted balancer must accept management clients");

    restarted_cluster.agents = agents;

    restarted_cluster
        .wait_for_agent_count(1)
        .await
        .expect("the agent must reconnect to the restarted balancer");

    restarted_cluster
        .shutdown()
        .await
        .expect("the restarted cluster must shut down cleanly");
}
