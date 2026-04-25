#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::start_in_process_cluster_with_qwen3::start_in_process_cluster_with_qwen3;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::request_params::ContinueFromRawPromptParams;
use reqwest::Client;

#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_long_and_short_prompts_complete_concurrently() -> Result<()> {
    let cluster = start_in_process_cluster_with_qwen3(2).await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let long_prompt = "Photosynthesis is the process by which green plants and certain other organisms transform light energy into chemical energy. During photosynthesis in green plants, light energy is captured and used to convert water, carbon dioxide, and minerals into oxygen and energy-rich organic compounds. Explain the process in detail:".to_owned();

    let long_stream = inference_client
        .post_continue_from_raw_prompt(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 20,
            raw_prompt: long_prompt,
        })
        .await?;

    let short_stream = inference_client
        .post_continue_from_raw_prompt(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 20,
            raw_prompt: "Hi".to_owned(),
        })
        .await?;

    let (long_collected, short_collected) = tokio::join!(
        collect_generated_tokens(long_stream),
        collect_generated_tokens(short_stream),
    );

    let long_collected = long_collected?;
    let short_collected = short_collected?;

    let long_tokens = long_collected
        .token_results
        .iter()
        .filter(|result| matches!(result, GeneratedTokenResult::Token(_)))
        .count();
    let short_tokens = short_collected
        .token_results
        .iter()
        .filter(|result| matches!(result, GeneratedTokenResult::Token(_)))
        .count();

    assert!(long_tokens > 0);
    assert!(short_tokens > 0);
    assert!(matches!(
        long_collected.token_results.last(),
        Some(GeneratedTokenResult::Done)
    ));
    assert!(matches!(
        short_collected.token_results.last(),
        Some(GeneratedTokenResult::Done)
    ));

    cluster.shutdown().await?;

    Ok(())
}
