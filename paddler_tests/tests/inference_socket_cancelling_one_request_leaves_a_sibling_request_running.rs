#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::collect_generated_tokens::collect_generated_tokens;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn inference_socket_cancelling_one_request_leaves_a_sibling_request_running() -> Result<()> {
    let mut cluster = start_cluster_with_qwen3(vec![AgentConfig::single(2)]).await?;

    let agent_id = cluster
        .agent_ids
        .first()
        .context("cluster must have one registered agent")?
        .clone();

    let cancelled_request_token = CancellationToken::new();
    let kept_request_token = CancellationToken::new();

    let mut cancelled_stream = cluster
        .client_inference
        .continue_from_raw_prompt(
            cancelled_request_token.clone(),
            ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: 500,
                raw_prompt: "Write a long story about an explorer".to_owned(),
            },
        )
        .await
        .map_err(anyhow::Error::new)?;

    let kept_stream = cluster
        .client_inference
        .continue_from_raw_prompt(
            kept_request_token.clone(),
            ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: 32,
                raw_prompt: "The capital of France is".to_owned(),
            },
        )
        .await
        .map_err(anyhow::Error::new)?;

    cluster
        .wait_for_slots_processing(&agent_id, 2)
        .await
        .context("both requests should occupy a slot")?;

    cancelled_stream
        .next()
        .await
        .context("the cancelled request must produce at least one message first")?
        .map_err(anyhow::Error::new)?;

    cancelled_request_token.cancel();

    assert!(
        cancelled_stream.next().await.is_none(),
        "the cancelled request must end its stream"
    );

    let kept_tokens = collect_generated_tokens(kept_stream).await?;

    assert!(
        !kept_tokens.token_results.is_empty(),
        "the sibling request sharing the same inference socket must keep generating tokens"
    );

    cluster
        .wait_for_slots_processing(&agent_id, 0)
        .await
        .context("both slots should be released once the sibling request completes")?;

    cluster.shutdown().await?;

    Ok(())
}
