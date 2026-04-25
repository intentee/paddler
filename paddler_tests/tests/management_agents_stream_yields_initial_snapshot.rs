#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_tests::start_subprocess_cluster_with_qwen3::start_subprocess_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn management_agents_stream_yields_initial_snapshot() -> Result<()> {
    let cluster = start_subprocess_cluster_with_qwen3(2, 1).await?;

    let mut stream = cluster
        .paddler_client
        .management()
        .get_agents_stream()
        .await
        .map_err(anyhow::Error::new)
        .context("agents stream should connect")?;

    let first_event = stream
        .next()
        .await
        .context("agents stream must produce at least one event")?
        .map_err(anyhow::Error::new)
        .context("first agents stream event should deserialize")?;

    assert!(
        !first_event.agents.is_empty(),
        "first agents stream event must contain at least one agent"
    );

    cluster.shutdown().await?;

    Ok(())
}
