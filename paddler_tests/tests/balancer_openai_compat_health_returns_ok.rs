#![cfg(feature = "tests_that_use_compiled_paddler")]

use anyhow::Context as _;
use anyhow::Result;
use paddler_tests::subprocess_cluster::SubprocessCluster;
use paddler_tests::subprocess_cluster_params::SubprocessClusterParams;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_openai_compat_health_returns_ok() -> Result<()> {
    let cluster = SubprocessCluster::start(SubprocessClusterParams {
        agent_count: 0,
        wait_for_slots_ready: false,
        ..SubprocessClusterParams::default()
    })
    .await?;

    let openai_health_url = cluster.addresses.compat_openai_base_url()?.join("health")?;

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
