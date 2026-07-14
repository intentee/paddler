use std::time::Duration;

use anyhow::Result;
use anyhow::anyhow;
use futures_util::StreamExt as _;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;
use tokio::time::timeout;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_shutdown_with_open_sse_subscriber_completes_within_one_second() -> Result<()> {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await?;

    let mut sse_stream = cluster
        .client_management
        .get_buffered_requests_stream()
        .await
        .map_err(anyhow::Error::new)?;

    let _first_snapshot = timeout(Duration::from_secs(1), sse_stream.next())
        .await
        .map_err(|elapsed| anyhow!("first SSE snapshot must arrive within 1s: {elapsed}"))?
        .ok_or_else(|| anyhow!("SSE stream closed before first snapshot"))?
        .map_err(anyhow::Error::new)?;

    timeout(Duration::from_secs(1), cluster.shutdown())
        .await
        .map_err(|elapsed| {
            anyhow!(
                "balancer in-process shutdown with an open SSE subscriber must complete within \
                 1s after cancel; got: {elapsed}"
            )
        })??;

    Ok(())
}
