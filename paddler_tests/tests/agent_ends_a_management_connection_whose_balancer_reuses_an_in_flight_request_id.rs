#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU16;

use serde_json::to_string;
use tokio_tungstenite::tungstenite::Message;

use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::jsonrpc::request_envelope::RequestEnvelope;
use paddler_messaging::management_socket::agent::message::Message as AgentJsonRpcMessage;
use paddler_messaging::management_socket::agent::notification::Notification as AgentJsonRpcNotification;
use paddler_messaging::management_socket::agent::notification_params::set_state_params::SetStateParams;
use paddler_messaging::management_socket::agent::request::Request as AgentJsonRpcRequest;
use paddler_messaging::reserved_request::ReservedRequest;
use paddler_test_cluster_harness::unending_generation::unending_generation;
use paddler_test_cluster_harness::untrusted_balancer_socket_server::UntrustedBalancerSocketServer;
use paddler_tests::qwen3_desired_state::qwen3_desired_state;

use crate::agent_runner_connected_to::agent_runner_connected_to;

#[tokio::test(flavor = "multi_thread")]
async fn agent_ends_a_management_connection_whose_balancer_reuses_an_in_flight_request_id() {
    let untrusted_balancer = UntrustedBalancerSocketServer::bind()
        .await
        .expect("the untrusted balancer must bind");
    let _agent_runner = agent_runner_connected_to(untrusted_balancer.management_addr, 1);
    let mut first_connection = untrusted_balancer
        .accept_agent()
        .await
        .expect("the agent must connect");
    let unending_generation_request = Message::text(
        to_string(&AgentJsonRpcMessage::Request(RequestEnvelope {
            id: "in-flight".to_owned(),
            request: AgentJsonRpcRequest::ContinueFromRawPrompt(ReservedRequest {
                params: unending_generation(),
                reserved_slots: NonZeroU16::MIN,
            }),
        }))
        .expect("the request must serialize"),
    );

    first_connection
        .send(Message::text(
            to_string(&AgentJsonRpcMessage::Notification(
                AgentJsonRpcNotification::SetState(Box::new(SetStateParams {
                    desired_state: AgentDesiredState::from(&qwen3_desired_state()),
                })),
            ))
            .expect("the desired state must serialize"),
        ))
        .await
        .expect("the desired state must be sent");
    first_connection
        .wait_for_the_agent_to_serve()
        .await
        .expect("the agent must serve its model");
    first_connection
        .send(unending_generation_request.clone())
        .await
        .expect("the request must be sent");
    first_connection
        .next_agent_response()
        .await
        .expect("the agent must start generating");
    first_connection
        .send(unending_generation_request)
        .await
        .expect("the reused request must be sent");
    first_connection.wait_for_the_agent_to_hang_up().await;

    untrusted_balancer
        .accept_agent()
        .await
        .expect("the agent must reconnect after ending the connection");
}
