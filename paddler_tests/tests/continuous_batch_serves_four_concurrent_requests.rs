#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use futures_util::future::try_join_all;
use tokio_util::sync::CancellationToken;

use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_serves_four_concurrent_requests() {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(4)])
        .await
        .expect("the cluster must start");

    let prompts = ["The sky is", "Roses are", "Once upon", "In the year"];

    let collected_results = try_join_all(prompts.into_iter().map(|prompt| {
        cluster.continue_from_raw_prompt(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::new(8).unwrap(),
                raw_prompt: prompt.to_owned(),
            },
        )
    }))
    .await
    .expect("every concurrent request must succeed");

    assert_eq!(collected_results.len(), 4);

    for collected in &collected_results {
        let token_count = collected
            .token_results
            .iter()
            .filter(|result| result.token_result.is_token())
            .count();

        assert!(
            token_count > 0,
            "every concurrent request must produce tokens"
        );
        collected
            .summary()
            .expect("the generation must finish with a summary");
    }

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
