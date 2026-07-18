#![cfg(feature = "tests_that_use_llms")]

use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use tokio_util::sync::CancellationToken;

const SLOT_RELEASE_OBSERVATION_WINDOW: Duration = Duration::from_secs(5);

#[tokio::test(flavor = "multi_thread")]
async fn http_inference_cancellation_releases_the_agent_slot() -> Result<()> {
    let mut cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)]).await?;

    let agent_id = cluster
        .agent_ids
        .first()
        .context("cluster must have one registered agent")?
        .clone();

    let cancellation_token = CancellationToken::new();

    let mut stream = cluster
        .continue_from_raw_prompt_stream(
            cancellation_token.clone(),
            &ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: 500,
                raw_prompt: "Write a long story about an explorer".to_owned(),
            },
        )
        .await?;

    stream
        .next()
        .await
        .context("inference stream must produce at least one message")?
        .map_err(anyhow::Error::new)?;

    cluster
        .wait_for_slots_processing(&agent_id, 1)
        .await
        .context("the request should occupy the only slot")?;

    cancellation_token.cancel();

    assert!(
        stream.next().await.is_none(),
        "a cancelled HTTP inference request must end its stream"
    );

    drop(stream);

    cluster
        .wait_for_slots_processing_within(&agent_id, 0, SLOT_RELEASE_OBSERVATION_WINDOW)
        .await
        .context("the slot should be released after the request is cancelled")?;

    cluster.shutdown().await?;

    Ok(())
}
