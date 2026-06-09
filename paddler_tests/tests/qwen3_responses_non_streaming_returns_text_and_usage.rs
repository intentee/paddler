#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use serde_json::Value;
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn qwen3_responses_non_streaming_returns_text_and_usage() -> Result<()> {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)]).await?;

    let response = cluster
        .openai_responses_non_streaming(&json!({
            "model": "qwen3-test",
            "input": "Say hi briefly.",
            "max_output_tokens": 600
        }))
        .await?;

    assert_eq!(
        response.get("status").and_then(Value::as_str),
        Some("completed"),
        "responses status must be completed: {response}"
    );

    let usage = response
        .get("usage")
        .ok_or_else(|| anyhow::anyhow!("responses response missing usage: {response}"))?;

    let input_tokens = usage
        .get("input_tokens")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("usage.input_tokens missing"))?;
    let output_tokens = usage
        .get("output_tokens")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("usage.output_tokens missing"))?;
    let total_tokens = usage
        .get("total_tokens")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("usage.total_tokens missing"))?;

    assert!(input_tokens > 0);
    assert!(output_tokens > 0);
    assert_eq!(total_tokens, input_tokens + output_tokens);

    let message_text = response
        .get("output")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("responses response missing output array"))?
        .iter()
        .find(|item| item.get("type").and_then(Value::as_str) == Some("message"))
        .and_then(|message| message.pointer("/content/0/text"))
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("responses output has no message text: {response}"))?;

    assert!(
        !message_text.is_empty(),
        "responses message text must not be empty"
    );

    cluster.shutdown().await?;

    Ok(())
}
