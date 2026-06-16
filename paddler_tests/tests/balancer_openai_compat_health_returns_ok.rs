use anyhow::Context as _;
use anyhow::Result;
use paddler_cluster::cluster::Cluster;
use paddler_cluster::cluster_params::ClusterParams;
use paddler_tests::in_process_cluster_backend::InProcessClusterBackend;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_openai_compat_health_returns_ok() -> Result<()> {
    let cluster = Cluster::start(
        &InProcessClusterBackend::default(),
        ClusterParams {
            agents: Vec::new(),
            wait_for_slots_ready: false,
            ..ClusterParams::default()
        },
    )
    .await?;

    let openai_health_url = cluster
        .balancer
        .addresses
        .compat_openai_base_url()?
        .join("health")?;

    let response = reqwest::get(openai_health_url)
        .await
        .context("failed to GET OpenAI compat /health")?;

    assert_eq!(response.status(), 200);

    let body = response
        .text()
        .await
        .context("failed to read response body")?;

    assert_eq!(body, "OK");

    cluster.shutdown().await?;

    Ok(())
}
