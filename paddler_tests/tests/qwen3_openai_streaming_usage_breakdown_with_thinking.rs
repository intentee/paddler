#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use serde_json::Value;
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn qwen3_openai_streaming_usage_breakdown_with_thinking() -> Result<()> {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)]).await?;

    let chunks = cluster
        .openai_chat_completion_streaming(&json!({
            "model": "qwen3-test",
            "messages": [{
                "role": "user",
                "content": "What is two plus two? Think briefly before answering."
            }],
            "stream": true,
            "stream_options": {"include_usage": true},
            "max_completion_tokens": 200
        }))
        .await?;

    let usage_chunk = chunks
        .iter()
        .rev()
        .find(|chunk| chunk.get("usage").is_some_and(|usage| !usage.is_null()))
        .ok_or_else(|| anyhow::anyhow!("no chunk contained usage information"))?;

    let prompt_tokens = usage_chunk
        .pointer("/usage/prompt_tokens")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("usage chunk missing prompt_tokens"))?;
    let completion_tokens = usage_chunk
        .pointer("/usage/completion_tokens")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("usage chunk missing completion_tokens"))?;
    let total_tokens = usage_chunk
        .pointer("/usage/total_tokens")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("usage chunk missing total_tokens"))?;

    assert!(prompt_tokens > 0);
    assert!(completion_tokens > 0);
    assert_eq!(total_tokens, prompt_tokens + completion_tokens);

    cluster.shutdown().await?;

    Ok(())
}
