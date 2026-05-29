use anyhow::Context as _;
use anyhow::Result;
use paddler_tests::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_inference_health_returns_ok() -> Result<()> {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await?;

    let inference_health_url = cluster
        .balancer
        .addresses
        .inference_base_url()?
        .join("health")?;

    let response = reqwest::get(inference_health_url)
        .await
        .context("failed to GET inference /health")?;

    assert_eq!(response.status(), 200);

    let body = response
        .text()
        .await
        .context("failed to read response body")?;

    assert_eq!(body, "OK");

    cluster.shutdown().await?;

    Ok(())
}
