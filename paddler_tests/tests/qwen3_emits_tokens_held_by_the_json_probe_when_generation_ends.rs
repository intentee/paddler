#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_messaging::grammar_constraint::GrammarConstraint;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn qwen3_emits_tokens_held_by_the_json_probe_when_generation_ends() -> Result<()> {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)]).await?;

    let collected = cluster
        .continue_from_raw_prompt(CancellationToken::new(), &ContinueFromRawPromptParams {
            grammar: Some(GrammarConstraint::Gbnf {
                grammar: r#"root ::= "{""#.to_owned(),
                root: "root".to_owned(),
            }),
            max_tokens: 10,
            raw_prompt: "<|im_start|>user\nReply with a single opening brace.<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n".to_owned(),
        })
        .await?;

    assert_eq!(
        collected.text, "{",
        "the token held by the JSON probe must still reach the client when generation ends"
    );

    cluster.shutdown().await?;

    Ok(())
}
