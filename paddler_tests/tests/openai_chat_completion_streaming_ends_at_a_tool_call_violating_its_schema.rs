#![cfg(feature = "tests_that_use_llms")]

use serde_json::Value;
use serde_json::json;

use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use paddler_tests::weather_tool_parameters_with_an_unsatisfiable_location::weather_tool_parameters_with_an_unsatisfiable_location;
use paddler_tests::weather_tool_question::WEATHER_TOOL_QUESTION;

#[tokio::test(flavor = "multi_thread")]
async fn openai_chat_completion_streaming_ends_at_a_tool_call_violating_its_schema() {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");

    let chunks = cluster
        .openai_chat_completion_streaming(&json!({
            "model": "qwen3-test",
            "messages": [{"role": "user", "content": WEATHER_TOOL_QUESTION}],
            "max_completion_tokens": 400,
            "stream": true,
            "tools": [{
                "type": "function",
                "function": {
                    "name": "get_weather",
                    "description": "Get the current weather for a location",
                    "parameters": weather_tool_parameters_with_an_unsatisfiable_location()
                }
            }]
        }))
        .await
        .expect("the OpenAI chat completion stream must be readable");

    assert_eq!(
        chunks
            .iter()
            .skip_while(|chunk| chunk.get("error").is_none())
            .map(|chunk| chunk["error"]["type"].clone())
            .collect::<Vec<Value>>(),
        vec![json!("server_error")]
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
