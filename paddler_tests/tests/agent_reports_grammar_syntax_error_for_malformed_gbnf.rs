#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::grammar_constraint::GrammarConstraint;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn agent_reports_grammar_syntax_error_for_malformed_gbnf() {
    let cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 2))
        .await
        .expect("the cluster must start");

    let collected = cluster
        .continue_from_raw_prompt(CancellationToken::new(), &ContinueFromRawPromptParams {
            grammar: Some(GrammarConstraint::Gbnf {
                grammar: r#"root ::= "unterminated"#.to_owned(),
                root: "root".to_owned(),
            }),
            max_tokens: NonZeroU32::new(10).unwrap(),
            raw_prompt:
                "<|im_start|>user\nSay hi.<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n"
                    .to_owned(),
        })
        .await.expect("the inference request must be accepted");

    let token_results = collected.into_token_results();

    assert!(matches!(
        token_results.as_slice(),
        [GeneratedTokenResult::GrammarSyntaxError(_)]
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
