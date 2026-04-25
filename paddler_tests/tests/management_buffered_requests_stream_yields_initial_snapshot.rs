#![cfg(feature = "tests_that_use_compiled_paddler")]

use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_tests::subprocess_cluster::SubprocessCluster;
use paddler_tests::subprocess_cluster_params::SubprocessClusterParams;

#[tokio::test(flavor = "multi_thread")]
async fn management_buffered_requests_stream_yields_initial_snapshot() -> Result<()> {
    let cluster = SubprocessCluster::start(SubprocessClusterParams {
        agent_count: 0,
        wait_for_slots_ready: false,
        ..SubprocessClusterParams::default()
    })
    .await?;

    let mut stream = cluster
        .paddler_client
        .management()
        .get_buffered_requests_stream()
        .await
        .map_err(anyhow::Error::new)
        .context("buffered requests stream should connect")?;

    let first_event = stream
        .next()
        .await
        .context("buffered requests stream must produce at least one event")?
        .map_err(anyhow::Error::new)
        .context("first event should deserialize")?;

    assert!(first_event.buffered_requests_current >= 0);

    cluster.shutdown().await?;

    Ok(())
}
