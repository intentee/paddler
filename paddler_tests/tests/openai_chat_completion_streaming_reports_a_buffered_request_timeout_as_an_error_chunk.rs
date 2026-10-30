use serde_json::json;

use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::short_buffered_request_timeout::SHORT_BUFFERED_REQUEST_TIMEOUT;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn openai_chat_completion_streaming_reports_a_buffered_request_timeout_as_an_error_chunk() {
    let cluster = start_cluster(ClusterParams {
        buffered_request_timeout: SHORT_BUFFERED_REQUEST_TIMEOUT,
        ..ClusterParams::without_agents()
    })
    .await
    .expect("a balancer without agents must start");

    let chunks = cluster
        .openai_chat_completion_streaming(&json!({
            "model": "test",
            "messages": [{"role": "user", "content": "hi"}],
            "stream": true,
        }))
        .await
        .expect("the OpenAI chat completion stream must be readable");

    assert_eq!(
        chunks,
        vec![json!({
            "error": {
                "code": null,
                "message": "Waiting for available slot timed out",
                "param": null,
                "type": "server_error"
            }
        })]
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
