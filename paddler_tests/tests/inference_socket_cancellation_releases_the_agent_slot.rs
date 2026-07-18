#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::observation_window::ObservationWindow;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn inference_socket_cancellation_releases_the_agent_slot() -> Result<()> {
    let mut cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)]).await?;

    let agent_id = cluster
        .agent_ids
        .first()
        .context("cluster must have one registered agent")?
        .clone();

    let cancellation_token = CancellationToken::new();

    let mut stream = cluster
        .client_inference
        .continue_from_raw_prompt(
            cancellation_token.clone(),
            ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: 500,
                raw_prompt: "Write a long story about an explorer".to_owned(),
            },
        )
        .await
        .map_err(anyhow::Error::new)?;

    stream
        .next()
        .await
        .context("inference stream must produce at least one message")?
        .map_err(anyhow::Error::new)?;

    cluster
        .wait_for_slots_processing(&agent_id, 1, ObservationWindow::model_load())
        .await
        .context("the request should occupy the only slot")?;

    cancellation_token.cancel();

    assert!(
        stream.next().await.is_none(),
        "a cancelled inference socket request must end its stream"
    );

    cluster
        .wait_for_slots_processing(&agent_id, 0, ObservationWindow::model_load())
        .await
        .context("the slot should be released after the request is cancelled")?;

    cluster.shutdown().await?;

    Ok(())
}
