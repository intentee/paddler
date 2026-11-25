use async_openai::error::ApiErrorResponse;
use async_openai::error::OpenAIError;
use http::StatusCode;
use serde_json::json;

use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_harness_error::ClusterHarnessError;
use paddler_cluster_harness::cluster_params::ClusterParams;

#[tokio::test(flavor = "multi_thread")]
async fn openai_responses_non_streaming_rejects_a_tool_that_requires_an_undeclared_property() {
    let cluster = Cluster::start_in_process(ClusterParams::without_agents())
        .await
        .expect("a balancer without agents must start");

    let rejection = cluster
        .openai_responses_non_streaming(&json!({
            "model": "test-model",
            "input": "hi",
            "tools": [
                {
                    "type": "function",
                    "name": "broken",
                    "parameters": {
                        "type": "object",
                        "properties": { "present": { "type": "string" } },
                        "required": ["absent"]
                    }
                }
            ]
        }))
        .await;

    assert!(matches!(
        rejection,
        Err(ClusterHarnessError::OpenAIRequestFailed(OpenAIError::ApiError(ApiErrorResponse {
            status_code,
            api_error,
        }))) if status_code == StatusCode::BAD_REQUEST
            && api_error.r#type.as_deref() == Some("invalid_request_error")
            && api_error.message == "Required field 'absent' not found in properties"
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
