#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use anyhow::Result;
use paddler_inference_parameters::inference_parameters::InferenceParameters;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::load_qwen3_after_request_is_buffered::load_qwen3_after_request_is_buffered;
use paddler_tests::start_cluster_without_model::start_cluster_without_model;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn agent_rejects_buffered_raw_prompt_after_switching_to_embeddings() -> Result<()> {
    let mut cluster = start_cluster_without_model(
        AgentConfig::uniform(1, 1),
        InferenceParameters::deterministic(),
    )
    .await?;

    let request = cluster.continue_from_raw_prompt(
        CancellationToken::new(),
        &ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: NonZeroU32::new(4).unwrap(),
            raw_prompt: "Hello".to_owned(),
        },
    );
    let collected = load_qwen3_after_request_is_buffered(&mut cluster, request, true).await?;

    assert_eq!(
        collected.into_token_results(),
        vec![GeneratedTokenResult::TokenGenerationDisabled(
            "Some(\"test-agent-0\"): token generation is disabled because this agent is running in embeddings-only mode"
                .to_owned()
        )]
    );

    cluster.shutdown().await?;

    Ok(())
}
