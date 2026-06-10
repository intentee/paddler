#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use anyhow::anyhow;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use serde_json::Value;
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn qwen3_openai_non_streaming_emits_tool_calls_for_function_tool() -> Result<()> {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)]).await?;

    let response = cluster
        .openai_chat_completion_non_streaming(&json!({
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
                            "location": {"type": "string", "description": "The city name"}
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
        .ok_or_else(|| anyhow!("response missing message.tool_calls: {response}"))?;

    assert_eq!(
        tool_calls.len(),
        1,
        "expected exactly one structured tool call in non-streaming response (got {})",
        tool_calls.len()
    );

    let first_call = &tool_calls[0];

    assert_eq!(
        first_call.pointer("/type").and_then(Value::as_str),
        Some("function"),
    );

    let id = first_call
        .pointer("/id")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("tool call missing id"))?;
    assert!(!id.is_empty(), "tool call id must not be empty");

    let function_name = first_call
        .pointer("/function/name")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("tool call missing function.name"))?;

    assert_eq!(function_name, "get_weather");

    let function_arguments = first_call
        .pointer("/function/arguments")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("tool call missing function.arguments"))?;

    let parsed_arguments: Value = serde_json::from_str(function_arguments)?;
    assert!(
        parsed_arguments.get("location").is_some(),
        "tool-call arguments JSON missing 'location' field: {function_arguments}"
    );

    let finish_reason = response
        .pointer("/choices/0/finish_reason")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("response missing finish_reason"))?;

    assert_eq!(finish_reason, "tool_calls");

    let completion_tokens = response
        .pointer("/usage/completion_tokens")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("response missing usage.completion_tokens"))?;
    assert!(completion_tokens > 0);

    cluster.shutdown().await?;

    Ok(())
}
