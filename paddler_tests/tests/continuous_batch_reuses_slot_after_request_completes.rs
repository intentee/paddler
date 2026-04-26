#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::start_in_process_cluster_with_qwen3::start_in_process_cluster_with_qwen3;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::request_params::ContinueFromRawPromptParams;
use reqwest::Client;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_reuses_slot_after_request_completes() -> Result<()> {
    let cluster = start_in_process_cluster_with_qwen3(1).await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let first_stream = inference_client
        .post_continue_from_raw_prompt(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 10,
            raw_prompt: "Hello world".to_owned(),
        })
        .await?;

    let first_collected = collect_generated_tokens(first_stream).await?;

    assert!(matches!(
        first_collected.token_results.last(),
        Some(GeneratedTokenResult::Done)
    ));

    let second_stream = inference_client
        .post_continue_from_raw_prompt(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 10,
            raw_prompt: "Goodbye world".to_owned(),
        })
        .await?;

    let second_collected = collect_generated_tokens(second_stream).await?;

    assert!(matches!(
        second_collected.token_results.last(),
        Some(GeneratedTokenResult::Done)
    ));

    let second_token_count = second_collected
        .token_results
        .iter()
        .filter(|result| matches!(result, GeneratedTokenResult::Token(_)))
        .count();

    assert!(
        second_token_count > 0,
        "second sequential request must reuse the slot and produce tokens"
    );

    cluster.shutdown().await?;

    Ok(())
}
