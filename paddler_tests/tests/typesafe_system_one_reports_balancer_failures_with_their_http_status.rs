use std::time::Duration;

use reqwest::StatusCode;
use serde_json::Value;
use serde_json::json;
use tokio::join;

use paddler_messaging::inference_mode::InferenceMode;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::raw_agent_socket::RawAgentSocket;
use paddler_tests::cluster_without_agents_serving::cluster_without_agents_serving;
use paddler_tests::serving_agent_status::serving_agent_status;
use paddler_tests::start_cluster::start_cluster;

const SHORT_BUFFERED_REQUEST_TIMEOUT: Duration = Duration::from_millis(50);

fn noul_request() -> Value {
    json!({"state": "state", "questions": {"paid": {"type": "noul"}}})
}

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_system_one_reports_balancer_failures_with_their_http_status() {
    let waiting_cluster = start_cluster(ClusterParams {
        buffered_request_timeout: SHORT_BUFFERED_REQUEST_TIMEOUT,
        ..cluster_without_agents_serving(InferenceMode::Decision)
    })
    .await
    .expect("a decision balancer without agents must start");

    assert_eq!(
        waiting_cluster
            .typesafe_system_one("waiting-request", &noul_request())
            .await
            .expect("the request must be answered")
            .status,
        StatusCode::GATEWAY_TIMEOUT
    );

    let mut raw_agent_socket = RawAgentSocket::connect(
        waiting_cluster.balancer.addresses.management,
        "vanishing-agent",
    )
    .await
    .expect("the raw agent must reach the agent socket");

    raw_agent_socket
        .register_with_status(serving_agent_status(InferenceMode::Decision))
        .await
        .expect("the raw agent must register");

    let disconnected_request = noul_request();
    let (disconnected_response, ()) = join!(
        waiting_cluster.typesafe_system_one("disconnected-request", &disconnected_request),
        async move {
            raw_agent_socket
                .next_request()
                .await
                .expect("the balancer must forward the decision to the agent");

            drop(raw_agent_socket);
        }
    );

    assert_eq!(
        disconnected_response
            .expect("the request must be answered")
            .status,
        StatusCode::BAD_GATEWAY
    );

    waiting_cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");

    let overflowing_cluster = start_cluster(ClusterParams {
        max_buffered_requests: 0,
        ..cluster_without_agents_serving(InferenceMode::Decision)
    })
    .await
    .expect("a decision balancer without agents must start");

    assert_eq!(
        overflowing_cluster
            .typesafe_system_one("overflowing-request", &noul_request())
            .await
            .expect("the request must be answered")
            .status,
        StatusCode::SERVICE_UNAVAILABLE
    );

    overflowing_cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
