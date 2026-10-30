use serde_json::json;

use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::short_buffered_request_timeout::SHORT_BUFFERED_REQUEST_TIMEOUT;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn openai_responses_streaming_reports_a_buffered_request_timeout_as_a_failed_response() {
    let cluster = start_cluster(ClusterParams {
        buffered_request_timeout: SHORT_BUFFERED_REQUEST_TIMEOUT,
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
            .map(|event| event["type"].clone())
            .collect::<Vec<_>>(),
        vec![
            json!("response.created"),
            json!("response.in_progress"),
            json!("response.failed")
        ]
    );
    assert_eq!(
        events[2]["response"]["error"],
        json!({
            "code": "server_error",
            "message": "Waiting for available slot timed out"
        })
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
