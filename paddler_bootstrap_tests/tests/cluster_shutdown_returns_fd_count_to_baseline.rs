use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_cluster_harness::resource_snapshot::ResourceSnapshot;

#[tokio::test(flavor = "multi_thread")]
async fn cluster_shutdown_returns_fd_count_to_baseline() {
    let before =
        ResourceSnapshot::try_from_self().expect("the process resource snapshot must be readable");

    let cluster = Cluster::start_in_process(ClusterParams {
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");
    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");

    let after =
        ResourceSnapshot::try_from_self().expect("the process resource snapshot must be readable");
    let diff = after.diff(&before);

    assert_eq!(
        diff.open_file_descriptors_grew_by, 0,
        "in-process cluster lifecycle leaked file descriptors"
    );
}
