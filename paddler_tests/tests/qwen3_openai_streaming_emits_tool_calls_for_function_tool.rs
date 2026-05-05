#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_tests::openai_chat_completions_client::OpenAIChatCompletionsClient;
use paddler_tests::start_in_process_cluster_with_qwen3::start_in_process_cluster_with_qwen3;
use reqwest::Client;
use serde_json::Value;
use serde_json::json;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn qwen3_openai_streaming_emits_tool_calls_for_function_tool() -> Result<()> {
    let cluster = start_in_process_cluster_with_qwen3(1).await?;
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

    let tool_call_argument_chunks = chunks
        .iter()
        .filter(|chunk| {
            chunk
                .pointer("/choices/0/delta/tool_calls/0/function/arguments")
                .and_then(Value::as_str)
                .is_some()
        })
        .count();

    assert!(
        tool_call_argument_chunks > 0,
        "expected at least one streaming chunk with delta.tool_calls function arguments (got {tool_call_argument_chunks})"
    );

    cluster.shutdown().await?;

    Ok(())
}
