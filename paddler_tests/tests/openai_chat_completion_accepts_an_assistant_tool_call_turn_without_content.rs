use async_openai::error::ApiErrorResponse;
use async_openai::error::OpenAIError;
use http::StatusCode;
use serde_json::json;

use paddler_test_cluster_harness::cluster_harness_error::ClusterHarnessError;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::short_buffered_request_timeout::SHORT_BUFFERED_REQUEST_TIMEOUT;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn openai_chat_completion_accepts_an_assistant_tool_call_turn_without_content() {
    let cluster = start_cluster(ClusterParams {
        buffered_request_timeout: SHORT_BUFFERED_REQUEST_TIMEOUT,
        ..ClusterParams::without_agents()
    })
    .await
    .expect("a balancer without agents must start");

    let outcome = cluster
        .openai_chat_completion_non_streaming(&json!({
            "model": "test-model",
            "messages": [
                { "role": "user", "content": "What does marker.txt contain?" },
                {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_0",
                        "type": "function",
                        "function": { "name": "read", "arguments": "{\"path\":\"marker.txt\"}" }
                    }]
                },
                { "role": "tool", "tool_call_id": "call_0", "content": "PADDLER-OPENCODE-MARKER" }
            ]
        }))
        .await;

    assert!(
        matches!(
            &outcome,
            Err(ClusterHarnessError::OpenAIRequestFailed(OpenAIError::ApiError(ApiErrorResponse {
                status_code,
                ..
            }))) if *status_code == StatusCode::GATEWAY_TIMEOUT
        ),
        "{outcome:?}"
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
