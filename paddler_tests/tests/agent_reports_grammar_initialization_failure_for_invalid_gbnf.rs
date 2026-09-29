#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use anyhow::anyhow;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::grammar_constraint::GrammarConstraint;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn agent_reports_grammar_initialization_failure_for_invalid_gbnf() -> Result<()> {
    let cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 2)).await?;

    let collected = cluster
        .continue_from_raw_prompt(CancellationToken::new(), &ContinueFromRawPromptParams {
            grammar: Some(GrammarConstraint::Gbnf {
                grammar: r#"root ::= "unterminated"#.to_owned(),
                root: "root".to_owned(),
            }),
            max_tokens: 10,
            raw_prompt:
                "<|im_start|>user\nSay hi.<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n"
                    .to_owned(),
        })
        .await?;

    let failure_message = collected
        .token_results
        .iter()
        .find_map(|event| match &event.token_result {
            GeneratedTokenResult::GrammarInitializationFailed(message) => Some(message.clone()),
            _ => None,
        })
        .ok_or_else(|| {
            anyhow!(
                "expected a GrammarInitializationFailed event for malformed GBNF; got:\n{}",
                collected.text
            )
        })?;

    assert!(
        failure_message.contains("grammar"),
        "the failure message should mention the grammar; got: {failure_message}"
    );

    cluster.shutdown().await?;

    Ok(())
}
