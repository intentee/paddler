#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn agent_serves_four_concurrent_clients_streaming_tokens() -> Result<()> {
    let cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 4)).await?;

    let prompts = ["The sky is", "Roses are", "Once upon", "In the year"];

    let client_tasks = prompts.into_iter().map(|prompt| {
        cluster.continue_from_raw_prompt(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: 8,
                raw_prompt: prompt.to_owned(),
            },
        )
    });

    let collected_results = futures_util::future::try_join_all(client_tasks).await?;

    assert_eq!(collected_results.len(), 4);

    for collected in collected_results {
        let token_count = collected
            .token_results
            .iter()
            .filter(|result| result.token_result.is_token())
            .count();

        assert!(token_count > 0);
    }

    cluster.shutdown().await?;

    Ok(())
}
