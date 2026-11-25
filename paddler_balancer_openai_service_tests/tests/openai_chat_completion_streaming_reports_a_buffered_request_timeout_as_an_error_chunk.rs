use serde_json::json;

use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_cluster_harness::short_buffered_request_timeout::short_buffered_request_timeout;

#[tokio::test(flavor = "multi_thread")]
async fn openai_chat_completion_streaming_reports_a_buffered_request_timeout_as_an_error_chunk() {
    let cluster = Cluster::start_in_process(ClusterParams {
        buffered_request_timeout: short_buffered_request_timeout(),
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
