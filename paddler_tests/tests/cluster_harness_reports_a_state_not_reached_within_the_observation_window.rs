use paddler_test_cluster_harness::cluster_harness_error::ClusterHarnessError;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::observation_window::ObservationWindow;
use paddler_test_cluster_harness::snapshots_stream::SnapshotsStream;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn cluster_harness_reports_a_state_not_reached_within_the_observation_window() {
    let mut cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a cluster without agents must start");

    let observation = cluster
        .wait_for_buffered_request_count(1, ObservationWindow::release())
        .await;

    assert!(matches!(
        observation,
        Err(ClusterHarnessError::ObservationWindowElapsed {
            snapshots_stream: SnapshotsStream::BufferedRequests,
            ..
        })
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
