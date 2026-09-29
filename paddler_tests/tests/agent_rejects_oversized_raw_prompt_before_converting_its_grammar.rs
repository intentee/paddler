#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use anyhow::Result;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::grammar_constraint::GrammarConstraint;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3_and_context_size::start_cluster_with_qwen3_and_context_size;
use tokio_util::sync::CancellationToken;

const SEQUENCE_CONTEXT_SIZE: u32 = 256;

#[tokio::test(flavor = "multi_thread")]
async fn agent_rejects_oversized_raw_prompt_before_converting_its_grammar() -> Result<()> {
    let cluster = start_cluster_with_qwen3_and_context_size(
        vec![AgentConfig::single(1)],
        SEQUENCE_CONTEXT_SIZE,
    )
    .await?;

    let collected = cluster
        .continue_from_raw_prompt(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: Some(GrammarConstraint::JsonSchema {
                    schema: "not a json schema".to_owned(),
                }),
                max_tokens: NonZeroU32::new(20).unwrap(),
                raw_prompt: "The quick brown fox jumps over the lazy dog. ".repeat(40),
            },
        )
        .await?;

    assert!(matches!(
        collected
            .token_results
            .iter()
            .map(|result| &result.token_result)
            .collect::<Vec<_>>()
            .as_slice(),
        [GeneratedTokenResult::PromptExceedsContextSize(details)]
            if details.sequence_context_size == SEQUENCE_CONTEXT_SIZE
    ));

    cluster.shutdown().await?;

    Ok(())
}
