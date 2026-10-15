use paddler_cli_tests::pausable_agent_cluster::PausableAgentCluster;
use paddler_cli_tests::pausable_agent_cluster_params::PausableAgentClusterParams;
use paddler_cli_tests::start_pausable_agent_cluster::start_pausable_agent_cluster;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;

#[tokio::test(flavor = "multi_thread")]
async fn a_balancer_drops_an_agent_that_stops_answering_pings() {
    let PausableAgentCluster {
        mut cluster,
        pausable_agent,
    } = start_pausable_agent_cluster(PausableAgentClusterParams {
        binary_path: env!("CARGO_BIN_EXE_paddler_cluster_node").to_owned(),
        desired_state: ClusterDesiredState::KeepStored,
        expected_slots_total: 0,
        slot_count: 1,
    })
    .await
    .expect("a cluster with a model-less agent must start");

    pausable_agent
        .signals
        .pause()
        .expect("the agent must stop answering pings");
    cluster
        .wait_for_agent_count(0)
        .await
        .expect("the balancer must drop the agent that stopped answering pings");

    pausable_agent
        .signals
        .resume()
        .expect("the agent must continue so that it can shut down");
    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
