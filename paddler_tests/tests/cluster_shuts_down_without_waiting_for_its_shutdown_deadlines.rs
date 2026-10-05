use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn cluster_shuts_down_without_waiting_for_its_shutdown_deadlines() {
    start_cluster(ClusterParams {
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start")
    .shutdown()
    .await
    .expect("the cluster must shut down cleanly");
}
