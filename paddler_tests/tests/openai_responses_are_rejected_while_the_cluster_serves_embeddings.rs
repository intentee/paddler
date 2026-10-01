use async_openai::error::ApiErrorResponse;
use async_openai::error::OpenAIError;
use http::StatusCode;
use serde_json::Value;
use serde_json::from_str;
use serde_json::json;

use paddler_inference_parameters::inference_parameters::InferenceParameters;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_test_cluster_harness::cluster_harness_error::ClusterHarnessError;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn openai_responses_are_rejected_while_the_cluster_serves_embeddings() {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        desired_state: Some(BalancerDesiredState {
            inference_parameters: InferenceParameters {
                enable_embeddings: true,
                ..InferenceParameters::default()
            },
            ..BalancerDesiredState::default()
        }),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a balancer serving embeddings must start");

    let rejection = cluster
        .openai_responses_non_streaming(&json!({ "model": "test-model", "input": "hi" }))
        .await;

    assert!(matches!(
        rejection,
        Err(ClusterHarnessError::OpenAIRequestFailed(OpenAIError::ApiError(ApiErrorResponse {
            status_code,
            api_error,
        }))) if status_code == StatusCode::NOT_IMPLEMENTED
            && from_str::<Value>(&api_error.message).is_ok_and(|error_body| error_body == json!({
                "error": {
                    "message": "Responses are disabled while the cluster is configured for embeddings",
                    "type": "server_error",
                    "param": null,
                    "code": null
                }
            }))
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
