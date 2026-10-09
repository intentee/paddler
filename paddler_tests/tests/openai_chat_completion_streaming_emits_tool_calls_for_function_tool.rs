#![cfg(feature = "tests_that_use_llms")]

use serde_json::Value;
use serde_json::from_str;
use serde_json::json;

use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn openai_chat_completion_streaming_emits_tool_calls_for_function_tool() {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");

    let chunks = cluster
        .openai_chat_completion_streaming(&json!({
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
        .await
        .expect("the OpenAI chat completion stream must succeed");

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
        .unwrap_or_else(|| {
            panic!("structured tool-call chunk missing function.name: {structured_chunk}")
        });

    assert_eq!(function_name, "get_weather");

    let function_arguments = structured_chunk
        .pointer("/choices/0/delta/tool_calls/0/function/arguments")
        .and_then(Value::as_str)
        .unwrap_or_else(|| {
            panic!("structured tool-call chunk missing function.arguments: {structured_chunk}")
        });

    let parsed_arguments: Value =
        from_str(function_arguments).expect("the output must be valid JSON");

    assert!(
        parsed_arguments.get("location").is_some(),
        "tool-call arguments JSON missing 'location' field: {function_arguments}"
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
