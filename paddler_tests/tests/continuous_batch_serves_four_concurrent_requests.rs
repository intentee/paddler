#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::token_result_with_producer::TokenResultWithProducer;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_serves_four_concurrent_requests() -> Result<()> {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(4)]).await?;

    let prompts = ["The sky is", "Roses are", "Once upon", "In the year"];

    let collected_results = futures_util::future::try_join_all(prompts.into_iter().map(|prompt| {
        cluster.continue_from_raw_prompt(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 8,
            raw_prompt: prompt.to_owned(),
        })
    }))
    .await?;

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
        assert!(matches!(
            collected.token_results.last(),
            Some(TokenResultWithProducer {
                token_result: GeneratedTokenResult::Done(_),
                ..
            })
        ));
    }

    cluster.shutdown().await?;

    Ok(())
}
