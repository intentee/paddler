#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler::grammar_constraint::GrammarConstraint;
use paddler::request_params::ContinueFromRawPromptParams;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn qwen3_json_schema_grammar_returns_valid_json() -> Result<()> {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)]).await?;

    let collected = cluster
        .continue_from_raw_prompt(&ContinueFromRawPromptParams {
            grammar: Some(GrammarConstraint::JsonSchema {
                schema: r#"{"type": "object", "properties": {"answer": {"type": "string"}}, "required": ["answer"]}"#.to_owned(),
            }),
            max_tokens: 50,
            raw_prompt: "<|im_start|>user\nWhat is 2+2?<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n".to_owned(),
        })
        .await?;

    let parsed: serde_json::Value = serde_json::from_str(&collected.text)?;

    assert!(
        parsed.get("answer").is_some(),
        "expected JSON with 'answer' field, got: {:?}",
        collected.text
    );

    cluster.shutdown().await?;

    Ok(())
}
