use std::time::Duration;

use reqwest::Client;
use reqwest::StatusCode;
use serde_json::Value;
use serde_json::json;
use tokio::join;

use paddler_balancer::compatibility::openai_service::openai_api_path::OpenAIApiPath;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_openai_response_format_validator::openai_validator::OpenAIValidator;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::raw_agent_socket::RawAgentSocket;
use paddler_tests::serving_agent_status::serving_agent_status;
use paddler_tests::start_cluster::start_cluster;

const SHORT_BUFFERED_REQUEST_TIMEOUT: Duration = Duration::from_millis(50);
const SHORT_INFERENCE_ITEM_TIMEOUT: Duration = Duration::from_millis(50);

fn agentless_cluster_params() -> ClusterParams {
    ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    }
}

#[cfg(test)]
async fn chat_completion_failure_status(cluster: &Cluster) -> StatusCode {
    let response = Client::new()
        .post(
            cluster
                .balancer
                .compat_openai_base_url()
                .expect("the cluster serves OpenAI")
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
    let status = response.status();

    OpenAIValidator::new()
        .expect("the OpenAI schema must load")
        .validate_error_response(
            &response
                .json::<Value>()
                .await
                .expect("the error body must be JSON"),
        )
        .expect("the error body must follow the OpenAI error schema");

    status
}

#[tokio::test(flavor = "multi_thread")]
async fn openai_chat_completion_reports_balancer_failures_with_their_http_status() {
    let waiting_cluster = start_cluster(ClusterParams {
        buffered_request_timeout: SHORT_BUFFERED_REQUEST_TIMEOUT,
        ..agentless_cluster_params()
    })
    .await
    .expect("a balancer without agents must start");

    assert_eq!(
        chat_completion_failure_status(&waiting_cluster).await,
        StatusCode::GATEWAY_TIMEOUT
    );

    let mut raw_agent_socket = RawAgentSocket::connect(
        waiting_cluster.balancer.addresses.management,
        "vanishing-agent",
    )
    .await
    .expect("the raw agent must reach the agent socket");

    raw_agent_socket
        .register_with_status(serving_agent_status(InferenceMode::TextGeneration))
        .await
        .expect("the raw agent must register");

    let (disconnected_status, ()) = join!(
        chat_completion_failure_status(&waiting_cluster),
        async move {
            raw_agent_socket
                .next_request()
                .await
                .expect("the balancer must forward the generation to the agent");

            drop(raw_agent_socket);
        }
    );

    assert_eq!(disconnected_status, StatusCode::BAD_GATEWAY);

    waiting_cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");

    let overflowing_cluster = start_cluster(ClusterParams {
        max_buffered_requests: 0,
        ..agentless_cluster_params()
    })
    .await
    .expect("a balancer without agents must start");

    assert_eq!(
        chat_completion_failure_status(&overflowing_cluster).await,
        StatusCode::SERVICE_UNAVAILABLE
    );

    overflowing_cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");

    let stalling_cluster = start_cluster(ClusterParams {
        inference_item_timeout: SHORT_INFERENCE_ITEM_TIMEOUT,
        ..agentless_cluster_params()
    })
    .await
    .expect("a balancer without agents must start");
    let mut stalling_agent_socket = RawAgentSocket::connect(
        stalling_cluster.balancer.addresses.management,
        "stalling-agent",
    )
    .await
    .expect("the raw agent must reach the agent socket");

    stalling_agent_socket
        .register_with_status(serving_agent_status(InferenceMode::TextGeneration))
        .await
        .expect("the raw agent must register");

    let (stalled_status, _stalling_agent_socket) = join!(
        chat_completion_failure_status(&stalling_cluster),
        async move {
            stalling_agent_socket
                .next_request()
                .await
                .expect("the balancer must forward the generation to the agent");

            stalling_agent_socket
        }
    );

    assert_eq!(stalled_status, StatusCode::GATEWAY_TIMEOUT);

    stalling_cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
