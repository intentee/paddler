#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use anyhow::Result;
use paddler_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::start_subprocess_cluster_with_qwen3::start_subprocess_cluster_with_qwen3;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::request_params::ContinueFromRawPromptParams;
use reqwest::Client;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn agent_serves_four_concurrent_clients_streaming_tokens() -> Result<()> {
    let cluster = start_subprocess_cluster_with_qwen3(4, 1).await?;

    let inference_base_url = cluster.addresses.inference_base_url()?;

    let prompts = ["The sky is", "Roses are", "Once upon", "In the year"];

    let client_tasks = prompts.into_iter().map(|prompt| {
        let inference_base_url = inference_base_url.clone();

        async move {
            let inference_client = InferenceHttpClient::new(Client::new(), inference_base_url);

            let stream = inference_client
                .post_continue_from_raw_prompt(&ContinueFromRawPromptParams {
                    grammar: None,
                    max_tokens: 8,
                    raw_prompt: prompt.to_owned(),
                })
                .await?;

            collect_generated_tokens(stream).await
        }
    });

    let collected_results = futures_util::future::try_join_all(client_tasks).await?;

    assert_eq!(collected_results.len(), 4);

    for collected in collected_results {
        let token_count = collected
            .token_results
            .iter()
            .filter(|result| matches!(result, GeneratedTokenResult::Token(_)))
            .count();

        assert!(token_count > 0);
    }

    cluster.shutdown().await?;

    Ok(())
}
