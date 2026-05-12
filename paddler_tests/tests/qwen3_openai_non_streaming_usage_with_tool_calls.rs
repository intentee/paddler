#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::openai_chat_completions_client::OpenAIChatCompletionsClient;
use paddler_tests::start_in_process_cluster_with_qwen3::start_in_process_cluster_with_qwen3;
use reqwest::Client;
use serde_json::Value;
use serde_json::json;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn qwen3_openai_non_streaming_usage_with_tool_calls() -> Result<()> {
    let cluster = start_in_process_cluster_with_qwen3(AgentConfig::single(1)).await?;
    let openai_client = OpenAIChatCompletionsClient::new(
        Client::new(),
        &cluster.addresses.compat_openai_base_url()?,
    )?;

    let response = openai_client
        .post_non_streaming(&json!({
            "model": "qwen3-test",
            "messages": [{
                "role": "user",
                "content": "What is the weather in Paris? Use the get_weather tool."
            }],
            "max_completion_tokens": 400,
            "tools": [{
                "type": "function",
                "function": {
                    "name": "get_weather",
                    "description": "Get the current weather for a location",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "location": {"type": "string"}
                        },
                        "required": ["location"],
                        "additionalProperties": false
                    }
                }
            }]
        }))
        .await?;

    let tool_calls = response
        .pointer("/choices/0/message/tool_calls")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("response missing message.tool_calls: {response}"))?;
    assert!(!tool_calls.is_empty());

    let usage = response
        .get("usage")
        .ok_or_else(|| anyhow::anyhow!("response missing usage: {response}"))?;

    let prompt_tokens = usage
        .get("prompt_tokens")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("usage.prompt_tokens missing"))?;
    let completion_tokens = usage
        .get("completion_tokens")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("usage.completion_tokens missing"))?;
    let total_tokens = usage
        .get("total_tokens")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("usage.total_tokens missing"))?;

    assert!(prompt_tokens > 0);
    // A request that produced a tool call must have spent tokens generating
    // the tool-call payload and any wrapping markers; completion_tokens
    // therefore cannot be zero.
    assert!(
        completion_tokens > 0,
        "expected non-zero completion_tokens for a tool-call response (got {completion_tokens})"
    );
    assert_eq!(total_tokens, prompt_tokens + completion_tokens);

    cluster.shutdown().await?;

    Ok(())
}
