#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_tests::openai_chat_completions_client::OpenAIChatCompletionsClient;
use paddler_tests::start_in_process_cluster_with_qwen3::start_in_process_cluster_with_qwen3;
use reqwest::Client;
use serde_json::Value;
use serde_json::json;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn qwen3_openai_non_streaming_emits_tool_calls_for_function_tool() -> Result<()> {
    let cluster = start_in_process_cluster_with_qwen3(1).await?;
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
        .ok_or_else(|| anyhow::anyhow!("response missing message.tool_calls: {response}"))?;

    assert!(
        !tool_calls.is_empty(),
        "expected at least one tool call in non-streaming response"
    );

    let first_call = &tool_calls[0];

    assert_eq!(
        first_call.pointer("/type").and_then(Value::as_str),
        Some("function")
    );
    assert!(
        first_call
            .pointer("/function/arguments")
            .and_then(Value::as_str)
            .is_some(),
        "tool call missing function.arguments"
    );

    let finish_reason = response
        .pointer("/choices/0/finish_reason")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("response missing finish_reason"))?;

    assert_eq!(finish_reason, "tool_calls");

    let completion_tokens = response
        .pointer("/usage/completion_tokens")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("response missing usage.completion_tokens"))?;
    assert!(completion_tokens > 0);

    cluster.shutdown().await?;

    Ok(())
}
