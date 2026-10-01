use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_harness_error::ClusterHarnessError;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::snapshots_stream::SnapshotsStream;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn cluster_harness_reports_the_end_of_a_snapshots_stream() {
    let Cluster {
        mut agents_watcher,
        balancer,
        ..
    } = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a cluster without agents must start");

    balancer
        .shutdown()
        .await
        .expect("the balancer must shut down cleanly");

    let observation = agents_watcher.until(|_snapshot| false).await;

    assert!(matches!(
        observation,
        Err(ClusterHarnessError::SnapshotsStreamClosed {
            snapshots_stream: SnapshotsStream::Agents
        })
    ));
}
