#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::grammar_constraint::GrammarConstraint;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn agent_reports_grammar_initialization_failure_for_invalid_gbnf() -> Result<()> {
    let cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 2)).await?;

    // `root ::= "unterminated` is syntactically broken GBNF (the string literal is
    // never closed). The `Gbnf` constraint is passed through verbatim, so the
    // malformed grammar only fails when `llama.cpp` compiles it inside
    // `GrammarSampler::into_llama_sampler`, exercising the agent's
    // grammar-initialization-failure path.
    let collected = cluster
        .continue_from_raw_prompt(&ContinueFromRawPromptParams {
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
            anyhow::anyhow!(
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
