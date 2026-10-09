use reqwest::Client;
use reqwest::StatusCode;
use serde_json::Value;
use serde_json::json;

use paddler_balancer::compatibility::openai_service::openai_api_path::OpenAIApiPath;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_openai_response_format_validator::openai_validator::OpenAIValidator;
use paddler_tests::cluster_without_agents_serving::cluster_without_agents_serving;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn openai_chat_completion_is_unavailable_while_the_cluster_serves_another_inference_mode() {
    let cluster = start_cluster(cluster_without_agents_serving(InferenceMode::Embeddings))
        .await
        .expect("a balancer serving embeddings must start");

    let response = Client::new()
        .post(
            cluster
                .balancer
                .compat_openai_base_url()
                .expect("the cluster serves OpenAI compatibility")
                .join(OpenAIApiPath::CHAT_COMPLETIONS)
                .expect("the path must join"),
        )
        .json(&json!({
            "model": "test-model",
            "messages": [{"role": "user", "content": "hi"}]
        }))
        .send()
        .await
        .expect("the chat completion request must be answered");

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    let error_body = response
        .json::<Value>()
        .await
        .expect("the error body must be JSON");

    OpenAIValidator::new()
        .expect("the OpenAI schema must load")
        .validate_error_response(&error_body)
        .expect("the error body must follow the OpenAI error schema");

    assert_eq!(
        error_body,
        json!({
            "error": {
                "message": "The cluster serves Embeddings, not TextGeneration",
                "type": "server_error",
                "param": null,
                "code": null
            }
        })
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
