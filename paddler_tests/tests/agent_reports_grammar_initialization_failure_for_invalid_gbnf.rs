#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_cluster::agent_config::AgentConfig;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::grammar_constraint::GrammarConstraint;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn agent_reports_grammar_initialization_failure_for_invalid_gbnf() -> Result<()> {
    let cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 2)).await?;

    let collected = cluster
        .inference_client.http().continue_from_raw_prompt_collected(&ContinueFromRawPromptParams {
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

    let reported_grammar_initialization_failure = collected.token_results.iter().any(|event| {
        matches!(
            event.token_result,
            GeneratedTokenResult::GrammarInitializationFailed(_)
        )
    });

    assert!(reported_grammar_initialization_failure);

    cluster.shutdown().await?;

    Ok(())
}
