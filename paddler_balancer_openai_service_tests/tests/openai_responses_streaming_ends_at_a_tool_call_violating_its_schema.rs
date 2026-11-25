#![cfg(feature = "tests_that_use_llms")]

use serde_json::Value;
use serde_json::json;

use paddler_balancer_openai_service_tests::weather_tool_parameters_with_an_unsatisfiable_location::weather_tool_parameters_with_an_unsatisfiable_location;
use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::start_cluster_with_qwen3::start_cluster_with_qwen3;
use paddler_request_params_fixture::weather_tool_question::weather_tool_question;

#[tokio::test(flavor = "multi_thread")]
async fn openai_responses_streaming_ends_at_a_tool_call_violating_its_schema() {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");

    let events = cluster
        .openai_responses_streaming(&json!({
            "model": "qwen3-test",
            "input": weather_tool_question(),
            "max_output_tokens": 400,
            "stream": true,
            "tools": [{
                "type": "function",
                "name": "get_weather",
                "description": "Get the current weather for a location",
                "parameters": weather_tool_parameters_with_an_unsatisfiable_location()
            }]
        }))
        .await
        .expect("the OpenAI responses stream must be readable");

    assert_eq!(
        events
            .iter()
            .skip_while(|event| event["type"] != "response.failed")
            .map(|event| event["type"].clone())
            .collect::<Vec<Value>>(),
        vec![json!("response.failed")]
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
