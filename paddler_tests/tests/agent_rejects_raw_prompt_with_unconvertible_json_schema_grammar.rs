#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::grammar_constraint::GrammarConstraint;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn agent_rejects_raw_prompt_with_unconvertible_json_schema_grammar() -> Result<()> {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)]).await?;

    let collected = cluster
        .continue_from_raw_prompt(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: Some(GrammarConstraint::JsonSchema {
                    schema: "not valid json at all".to_owned(),
                }),
                max_tokens: 20,
                raw_prompt: "The capital of France is".to_owned(),
            },
        )
        .await?;

    assert!(
        !collected
            .token_results
            .iter()
            .any(|result| result.token_result.is_token())
    );
    assert!(collected.token_results.iter().any(|result| matches!(
        &result.token_result,
        GeneratedTokenResult::GrammarSyntaxError(message)
            if message.contains("Failed to convert JSON schema to grammar")
    )));

    cluster.shutdown().await?;

    Ok(())
}
