#![cfg(feature = "tests_that_use_llms")]

use std::time::Duration;

use reqwest::StatusCode;
use serde_json::Value;

use paddler_balancer::compatibility::openai_service::openai_api_path::OpenAIApiPath;
use paddler_openai_response_format_validator::openai_validator::OpenAIValidator;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::minimal_openai_chat_completion::minimal_openai_chat_completion;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn openai_chat_completion_reports_a_stalled_generation_as_a_gateway_timeout() {
    let cluster = start_cluster(ClusterParams {
        agents: vec![AgentConfig::single(1)],
        desired_state: ClusterDesiredState::Apply(Box::new(qwen3_0_6b().into_desired_state())),
        inference_item_timeout: Duration::ZERO,
        ..ClusterParams::default()
    })
    .await
    .expect("a text generation cluster must start");

    let response = minimal_openai_chat_completion(
        cluster
            .balancer
            .compat_openai_url(OpenAIApiPath::CHAT_COMPLETIONS)
            .expect("the balancer must serve the OpenAI compatibility service"),
    )
    .await
    .expect("the balancer must answer the chat completion");

    assert_eq!(response.status(), StatusCode::GATEWAY_TIMEOUT);

    OpenAIValidator::new()
        .expect("the OpenAI specification must load")
        .validate_error_response(
            &response
                .json::<Value>()
                .await
                .expect("the error body must be JSON"),
        )
        .expect("the error body must be an OpenAI error");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
