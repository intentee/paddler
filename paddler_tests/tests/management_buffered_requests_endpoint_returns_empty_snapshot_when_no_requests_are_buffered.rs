use anyhow::Context as _;
use anyhow::Result;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn management_buffered_requests_endpoint_returns_empty_snapshot_when_no_requests_are_buffered()
-> Result<()> {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await?;

    let snapshot = cluster
        .client_management
        .get_buffered_requests()
        .await
        .map_err(anyhow::Error::new)
        .context("failed to GET /api/v1/buffered_requests")?;

    assert_eq!(snapshot.buffered_requests_current, 0);

    cluster.shutdown().await?;

    Ok(())
}
