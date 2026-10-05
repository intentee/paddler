use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn management_agents_stream_ends_when_cancelled() {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let cancellation_token = CancellationToken::new();

    let mut stream = cluster
        .client_management
        .get_agents_stream(cancellation_token.clone())
        .await
        .expect("the agents stream must open");

    stream
        .next()
        .await
        .expect("the agents stream must yield an initial snapshot")
        .expect("the message must be readable");

    cancellation_token.cancel();

    assert!(
        stream.next().await.is_none(),
        "a cancelled agents stream must end"
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
