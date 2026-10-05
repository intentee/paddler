#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn agent_streams_tokens_from_raw_prompt() -> Result<()> {
    let cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 2)).await?;

    let collected = cluster
        .continue_from_raw_prompt(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: 10,
                raw_prompt: "The capital of France is".to_owned(),
            },
        )
        .await?;

    let token_count = collected
        .token_results
        .iter()
        .filter(|result| result.token_result.is_token())
        .count();

    assert!(token_count > 0);

    cluster.shutdown().await?;

    Ok(())
}
