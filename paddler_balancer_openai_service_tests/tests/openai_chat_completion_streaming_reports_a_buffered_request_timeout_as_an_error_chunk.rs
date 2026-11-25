use serde_json::json;

use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_cluster_harness::constants::SHORT_BUFFERED_REQUEST_TIMEOUT;

#[tokio::test(flavor = "multi_thread")]
async fn openai_chat_completion_streaming_reports_a_buffered_request_timeout_as_an_error_chunk() {
    let cluster = Cluster::start_in_process(ClusterParams {
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
