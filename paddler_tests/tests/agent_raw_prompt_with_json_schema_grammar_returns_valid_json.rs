#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use serde_json::Value;
use serde_json::from_str;
use tokio_util::sync::CancellationToken;

use paddler_messaging::grammar_constraint::GrammarConstraint;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn agent_raw_prompt_with_json_schema_grammar_returns_valid_json() {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");

    let collected = cluster
        .continue_from_raw_prompt(CancellationToken::new(), &ContinueFromRawPromptParams {
            grammar: Some(GrammarConstraint::JsonSchema {
                schema: r#"{"type": "object", "properties": {"answer": {"type": "string"}}, "required": ["answer"]}"#.to_owned(),
            }),
            max_tokens: NonZeroU32::new(50).unwrap(),
            raw_prompt: "<|im_start|>user\nWhat is 2+2?<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n".to_owned(),
        })
        .await.expect("the inference request must be accepted");

    let parsed: Value = from_str(&collected.text).expect("the output must be valid JSON");

    assert!(
        parsed.get("answer").is_some(),
        "expected JSON with 'answer' field, got: {:?}",
        collected.text
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
