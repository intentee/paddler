#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use anyhow::anyhow;
use futures_util::StreamExt as _;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_releases_slots_on_shutdown_with_active_request() -> Result<()> {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)]).await?;

    let mut stream = cluster
        .continue_from_raw_prompt_stream(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: 500,
                raw_prompt: "Write a long essay".to_owned(),
            },
        )
        .await?;

    let _first_message = stream
        .next()
        .await
        .ok_or_else(|| anyhow!("inference stream must yield at least one message"))?;

    drop(stream);

    cluster.shutdown().await?;

    Ok(())
}
