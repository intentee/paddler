use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_params::ClusterParams;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_shutdown_does_not_wait_for_an_open_sse_subscriber() {
    let cluster = Cluster::start_in_process(ClusterParams::without_agents())
        .await
        .expect("the cluster must start");
    let mut sse_stream = cluster
        .client_management
        .get_buffered_requests_stream(CancellationToken::new())
        .await
        .expect("the buffered requests stream must open");

    sse_stream
        .next()
        .await
        .expect("the SSE stream must deliver its first snapshot")
        .expect("the message must be readable");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
