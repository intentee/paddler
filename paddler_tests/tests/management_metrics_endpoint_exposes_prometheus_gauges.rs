#![cfg(feature = "tests_that_use_compiled_paddler")]

use anyhow::Context as _;
use anyhow::Result;
use paddler_tests::subprocess_cluster::SubprocessCluster;
use paddler_tests::subprocess_cluster_params::SubprocessClusterParams;

#[tokio::test(flavor = "multi_thread")]
async fn management_metrics_endpoint_exposes_prometheus_gauges() -> Result<()> {
    let cluster = SubprocessCluster::start(SubprocessClusterParams {
        agent_count: 0,
        wait_for_slots_ready: false,
        ..SubprocessClusterParams::default()
    })
    .await?;

    let metrics = cluster
        .paddler_client
        .management()
        .get_metrics()
        .await
        .map_err(anyhow::Error::new)
        .context("get_metrics should succeed")?;

    assert!(
        metrics.contains("slots_processing"),
        "metrics must contain slots_processing gauge"
    );
    assert!(
        metrics.contains("slots_total"),
        "metrics must contain slots_total gauge"
    );

    cluster.shutdown().await?;

    Ok(())
}
