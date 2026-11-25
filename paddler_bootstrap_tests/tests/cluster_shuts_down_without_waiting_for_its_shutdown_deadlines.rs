use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_params::ClusterParams;

#[tokio::test(flavor = "multi_thread")]
async fn cluster_shuts_down_without_waiting_for_its_shutdown_deadlines() {
    Cluster::start_in_process(ClusterParams {
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start")
    .shutdown()
    .await
    .expect("the cluster must shut down cleanly");
}
