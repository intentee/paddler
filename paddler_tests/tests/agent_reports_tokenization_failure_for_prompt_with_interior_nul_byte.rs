#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_cluster::agent_config::AgentConfig;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn agent_reports_tokenization_failure_for_prompt_with_interior_nul_byte() -> Result<()> {
    let cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 2)).await?;

    let collected = cluster
        .inference_client
        .http()
        .continue_from_raw_prompt_collected(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 10,
            raw_prompt: "hello\u{0}world".to_owned(),
        })
        .await?;

    let reported_tokenization_failure = collected
        .token_results
        .iter()
        .any(|event| matches!(event.token_result, GeneratedTokenResult::SamplerError(_)));

    assert!(reported_tokenization_failure);

    cluster.shutdown().await?;

    Ok(())
}
