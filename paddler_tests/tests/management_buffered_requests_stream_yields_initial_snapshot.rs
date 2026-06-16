use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_cluster::cluster::Cluster;
use paddler_cluster::cluster_params::ClusterParams;
use paddler_tests::in_process_cluster_backend::InProcessClusterBackend;

#[tokio::test(flavor = "multi_thread")]
async fn management_buffered_requests_stream_yields_initial_snapshot() -> Result<()> {
    let cluster = Cluster::start(
        &InProcessClusterBackend::default(),
        ClusterParams {
            agents: Vec::new(),
            wait_for_slots_ready: false,
            ..ClusterParams::default()
        },
    )
    .await?;

    let mut stream = cluster
        .management_client
        .buffered_requests_stream()
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
