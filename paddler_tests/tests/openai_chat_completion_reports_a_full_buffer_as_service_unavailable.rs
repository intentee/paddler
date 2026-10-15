use reqwest::StatusCode;
use serde_json::Value;

use paddler_balancer::compatibility::openai_service::openai_api_path::OpenAIApiPath;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_openai_response_format_validator::openai_validator::OpenAIValidator;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::cluster_without_agents_serving::cluster_without_agents_serving;
use paddler_tests::minimal_openai_chat_completion::minimal_openai_chat_completion;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn openai_chat_completion_reports_a_full_buffer_as_service_unavailable() {
    let cluster = start_cluster(ClusterParams {
        max_buffered_requests: 0,
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

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

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
