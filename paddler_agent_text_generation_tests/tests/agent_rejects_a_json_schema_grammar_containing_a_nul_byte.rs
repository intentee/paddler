#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::start_cluster_with_qwen3::start_cluster_with_qwen3;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::grammar_constraint::GrammarConstraint;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;

#[tokio::test(flavor = "multi_thread")]
async fn agent_rejects_a_json_schema_grammar_containing_a_nul_byte() {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");

    let collected = cluster
        .continue_from_raw_prompt(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: Some(GrammarConstraint::JsonSchema {
                    schema: "{\"type\": \"string\"}\0".to_owned(),
                }),
                max_tokens: NonZeroU32::new(20).unwrap(),
                raw_prompt: "The capital of France is".to_owned(),
            },
        )
        .await
        .expect("the inference request must be accepted");

    assert!(matches!(
        collected.into_token_results().as_slice(),
        [GeneratedTokenResult::GrammarSyntaxError(_)]
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
