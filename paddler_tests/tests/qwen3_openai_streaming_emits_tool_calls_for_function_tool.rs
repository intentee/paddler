#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::openai_chat_completions_client::OpenAIChatCompletionsClient;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use reqwest::Client;
use serde_json::Value;
use serde_json::json;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn qwen3_openai_streaming_emits_tool_calls_for_function_tool() -> Result<()> {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)]).await?;
    let openai_client = OpenAIChatCompletionsClient::new(
        Client::new(),
        &cluster.addresses.compat_openai_base_url()?,
    )?;

    let chunks = openai_client
        .post_streaming(&json!({
            "model": "qwen3-test",
            "messages": [{
                "role": "user",
                "content": "What is the weather in Paris? Use the get_weather tool."
            }],
            "stream": true,
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

    let chunks_with_tool_calls: Vec<&Value> = chunks
        .iter()
        .filter(|chunk| {
            chunk
                .pointer("/choices/0/delta/tool_calls")
                .and_then(Value::as_array)
                .is_some_and(|calls| !calls.is_empty())
        })
        .collect();

    assert_eq!(
        chunks_with_tool_calls.len(),
        1,
        "expected exactly one structured tool-call chunk per call (got {})",
        chunks_with_tool_calls.len()
    );

    let structured_chunk = chunks_with_tool_calls[0];

    let function_name = structured_chunk
        .pointer("/choices/0/delta/tool_calls/0/function/name")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            anyhow::anyhow!("structured tool-call chunk missing function.name: {structured_chunk}")
        })?;

    assert_eq!(function_name, "get_weather");

    let function_arguments = structured_chunk
        .pointer("/choices/0/delta/tool_calls/0/function/arguments")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "structured tool-call chunk missing function.arguments: {structured_chunk}"
            )
        })?;

    let parsed_arguments: Value = serde_json::from_str(function_arguments)?;

    assert!(
        parsed_arguments.get("location").is_some(),
        "tool-call arguments JSON missing 'location' field: {function_arguments}"
    );

    cluster.shutdown().await?;

    Ok(())
}
