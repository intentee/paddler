#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use llama_cpp_bindings::error::SamplerApplyError;
use tokio_util::sync::CancellationToken;

use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::grammar_constraint::GrammarConstraint;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

const MAX_TOKENS: NonZeroU32 = NonZeroU32::new(10).unwrap();

#[tokio::test(flavor = "multi_thread")]
async fn agent_reports_a_grammar_no_token_can_satisfy_as_rejected_model_output() {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");

    let collected = cluster
        .continue_from_raw_prompt(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: Some(GrammarConstraint::Gbnf {
                    grammar: r#"root ::= "\x00""#.to_owned(),
                    root: "root".to_owned(),
                }),
                max_tokens: MAX_TOKENS,
                raw_prompt: "Paddler".to_owned(),
            },
        )
        .await
        .expect("the inference request must be accepted");

    assert_eq!(
        collected.into_token_results(),
        vec![GeneratedTokenResult::GrammarRejectedModelOutput(
            SamplerApplyError::EveryCandidateExcluded.to_string()
        )]
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
