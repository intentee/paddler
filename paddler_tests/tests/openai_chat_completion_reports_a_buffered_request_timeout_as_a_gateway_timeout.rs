use std::time::Duration;

use reqwest::StatusCode;
use serde_json::Value;

use paddler_balancer::compatibility::openai_service::openai_api_path::OpenAIApiPath;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_openai_response_format_validator::openai_validator::OpenAIValidator;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::cluster_without_agents_serving::cluster_without_agents_serving;
use paddler_tests::minimal_openai_chat_completion::minimal_openai_chat_completion;
use paddler_tests::start_cluster::start_cluster;

const SHORT_BUFFERED_REQUEST_TIMEOUT: Duration = Duration::from_millis(50);

#[tokio::test(flavor = "multi_thread")]
async fn openai_chat_completion_reports_a_buffered_request_timeout_as_a_gateway_timeout() {
    let cluster = start_cluster(ClusterParams {
        buffered_request_timeout: SHORT_BUFFERED_REQUEST_TIMEOUT,
        ..cluster_without_agents_serving(InferenceMode::TextGeneration)
    })
    .await
    .expect("a balancer without agents must start");

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
