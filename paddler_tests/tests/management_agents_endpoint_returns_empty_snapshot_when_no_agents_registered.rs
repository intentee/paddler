use tokio_util::sync::CancellationToken;

use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn management_agents_endpoint_returns_empty_snapshot_when_no_agents_registered() {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let snapshot = cluster
        .client_management
        .get_agents(CancellationToken::new())
        .await
        .expect("the agents snapshot must be served");

    assert!(snapshot.agents.is_empty());

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
