#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use anyhow::Context as _;
use anyhow::Result;
use paddler_tests::start_subprocess_cluster_with_qwen3::start_subprocess_cluster_with_qwen3;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn management_reports_zero_download_progress_after_load_complete() -> Result<()> {
    let cluster = start_subprocess_cluster_with_qwen3(2, 1).await?;

    let snapshot = cluster
        .paddler_client
        .management()
        .get_agents()
        .await
        .map_err(anyhow::Error::new)
        .context("get_agents should succeed")?;

    assert!(
        !snapshot.agents.is_empty(),
        "cluster should have at least one registered agent"
    );

    let agent = &snapshot.agents[0];

    assert_eq!(
        agent.download_current, 0,
        "download_current should be 0 once the model is loaded"
    );
    assert_eq!(
        agent.download_total, 0,
        "download_total should be 0 once the model is loaded"
    );

    cluster.shutdown().await?;

    Ok(())
}
