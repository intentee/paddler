#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::start_in_process_cluster_with_qwen3::start_in_process_cluster_with_qwen3;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::request_params::ContinueFromRawPromptParams;
use reqwest::Client;

#[tokio::test(flavor = "multi_thread")]
async fn qwen3_generates_tokens_from_raw_prompt() -> Result<()> {
    let cluster = start_in_process_cluster_with_qwen3(1).await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let stream = inference_client
        .post_continue_from_raw_prompt(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 30,
            raw_prompt: "<|im_start|>user\nHow can I make a cat happy?<|im_end|>\n<|im_start|>assistant\n".to_owned(),
        })
        .await?;

    let collected = collect_generated_tokens(stream).await?;

    let token_count = collected
        .token_results
        .iter()
        .filter(|result| matches!(result, GeneratedTokenResult::Token(_)))
        .count();

    assert!(token_count > 0);
    assert!(matches!(
        collected.token_results.last(),
        Some(GeneratedTokenResult::Done)
    ));

    cluster.shutdown().await?;

    Ok(())
}
