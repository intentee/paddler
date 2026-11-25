use serde_json::json;

use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_cluster_harness::short_buffered_request_timeout::short_buffered_request_timeout;

#[tokio::test(flavor = "multi_thread")]
async fn openai_responses_streaming_reports_a_buffered_request_timeout_as_a_failed_response() {
    let cluster = Cluster::start_in_process(ClusterParams {
        buffered_request_timeout: short_buffered_request_timeout(),
        ..ClusterParams::without_agents()
    })
    .await
    .expect("a balancer without agents must start");

    let events = cluster
        .openai_responses_streaming(&json!({
            "model": "test",
            "input": "hi",
            "stream": true,
        }))
        .await
        .expect("the OpenAI responses stream must be readable");

    assert_eq!(
        events
            .iter()
            .map(|event| event.pointer("/type"))
            .collect::<Vec<_>>(),
        vec![
            Some(&json!("response.created")),
            Some(&json!("response.in_progress")),
            Some(&json!("response.failed"))
        ]
    );
    assert_eq!(
        events
            .get(2)
            .and_then(|failed_event| failed_event.pointer("/response/error")),
        Some(&json!({
            "code": "server_error",
            "message": "Waiting for available slot timed out"
        }))
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
