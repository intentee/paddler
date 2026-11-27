#![cfg(feature = "tests_that_use_llms")]

use reqwest::StatusCode;
use serde_json::Value;
use serde_json::json;

use paddler_balancer_openai_service::constants::OPENAI_API_PATH_CHAT_COMPLETIONS;
use paddler_balancer_openai_service_tests::weather_tool_parameters_with_an_unsatisfiable_location::weather_tool_parameters_with_an_unsatisfiable_location;
use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_cluster_harness::post_openai_request::post_openai_request;
use paddler_model_card_fixture::qwen3_0_6b::qwen3_0_6b;
use paddler_openai_schema_validator::openai_validator::OpenAIValidator;
use paddler_request_params_fixture::weather_tool_question::weather_tool_question;

#[tokio::test(flavor = "multi_thread")]
async fn openai_chat_completion_reports_a_tool_call_violating_its_schema_as_a_server_error() {
    let cluster = Cluster::start_in_process(ClusterParams {
        agents: vec![AgentConfig::single(1)],
        desired_state: ClusterDesiredState::Apply(Box::new(qwen3_0_6b().into_desired_state())),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let response = post_openai_request(
        cluster
            .balancer
            .compat_openai_url(OPENAI_API_PATH_CHAT_COMPLETIONS)
            .expect("the balancer must serve the OpenAI compatibility service"),
        &json!({
            "model": "qwen3-test",
            "messages": [{
                "role": "user",
                "content": weather_tool_question()
            }],
            "max_completion_tokens": 400,
            "tools": [{
                "type": "function",
                "function": {
                    "name": "get_weather",
                    "description": "Get the current weather for a location",
                    "parameters": weather_tool_parameters_with_an_unsatisfiable_location()
                }
            }]
        }),
    )
    .await
    .expect("the balancer must answer the chat completion");

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let error_body = response
        .json::<Value>()
        .await
        .expect("the error body must be JSON");

    OpenAIValidator::new()
        .expect("the OpenAI specification must load")
        .validate_error_response(&error_body)
        .expect("the error body must be an OpenAI error");

    assert_eq!(
        error_body.pointer("/error/type").and_then(Value::as_str),
        Some("server_error")
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
