use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn management_buffered_requests_stream_yields_initial_snapshot() {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let mut stream = cluster
        .client_management
        .get_buffered_requests_stream(CancellationToken::new())
        .await
        .expect("buffered requests stream should connect");

    let first_event = stream
        .next()
        .await
        .expect("buffered requests stream must produce at least one event")
        .expect("first event should deserialize");

    assert_eq!(first_event.buffered_requests_current, 0);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
