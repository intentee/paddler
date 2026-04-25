#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::start_in_process_cluster_with_qwen3::start_in_process_cluster_with_qwen3;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::request_params::ContinueFromRawPromptParams;
use reqwest::Client;

#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_serves_four_concurrent_requests() -> Result<()> {
    let cluster = start_in_process_cluster_with_qwen3(4).await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let prompts = ["The sky is", "Roses are", "Once upon", "In the year"];

    let stream_results = futures_util::future::try_join_all(prompts.into_iter().map(|prompt| {
        let inference_client = &inference_client;

        async move {
            inference_client
                .post_continue_from_raw_prompt(&ContinueFromRawPromptParams {
                    grammar: None,
                    max_tokens: 8,
                    raw_prompt: prompt.to_owned(),
                })
                .await
        }
    }))
    .await?;

    let collect_tasks = stream_results.into_iter().map(collect_generated_tokens);

    let collected_results = futures_util::future::try_join_all(collect_tasks).await?;

    assert_eq!(collected_results.len(), 4);

    for collected in &collected_results {
        let token_count = collected
            .token_results
            .iter()
            .filter(|result| matches!(result, GeneratedTokenResult::Token(_)))
            .count();

        assert!(token_count > 0, "every concurrent request must produce tokens");
        assert!(matches!(
            collected.token_results.last(),
            Some(GeneratedTokenResult::Done)
        ));
    }

    cluster.shutdown().await?;

    Ok(())
}
