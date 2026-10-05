use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn management_agents_stream_ends_when_cancelled() -> Result<()> {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await?;

    let cancellation_token = CancellationToken::new();

    let mut stream = cluster
        .client_management
        .get_agents_stream(cancellation_token.clone())
        .await
        .map_err(anyhow::Error::new)
        .context("failed to open /api/v1/agents/stream")?;

    stream
        .next()
        .await
        .context("the agents stream must yield an initial snapshot")?
        .map_err(anyhow::Error::new)?;

    cancellation_token.cancel();

    assert!(
        stream.next().await.is_none(),
        "a cancelled agents stream must end"
    );

    cluster.shutdown().await?;

    Ok(())
}
