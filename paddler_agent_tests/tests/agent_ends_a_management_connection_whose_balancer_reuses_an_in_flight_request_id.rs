#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU16;

use log::Level;
use serde_json::to_string;
use tokio_tungstenite::tungstenite::Message;

use paddler_agent_tests::agent_connected_to_an_untrusted_balancer::AgentConnectedToAnUntrustedBalancer;
use paddler_balancer_desired_state_fixture::qwen3_desired_state::qwen3_desired_state;
use paddler_capturing_logger::captured_record::CapturedRecord;
use paddler_capturing_logger::capturing_logger::CapturingLogger;
use paddler_cluster_harness::unending_generation::unending_generation;
use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::jsonrpc::request_envelope::RequestEnvelope;
use paddler_messaging::management_socket::agent::message::Message as AgentJsonRpcMessage;
use paddler_messaging::management_socket::agent::notification::Notification as AgentJsonRpcNotification;
use paddler_messaging::management_socket::agent::notification_params::set_state_params::SetStateParams;
use paddler_messaging::management_socket::agent::request::Request as AgentJsonRpcRequest;
use paddler_messaging::reserved_request::ReservedRequest;

#[tokio::test(flavor = "multi_thread")]
async fn agent_ends_a_management_connection_whose_balancer_reuses_an_in_flight_request_id() {
    CapturingLogger::install();

    let AgentConnectedToAnUntrustedBalancer {
        agent_runner: _agent_runner,
        connection: mut first_connection,
        untrusted_balancer,
    } = AgentConnectedToAnUntrustedBalancer::connect()
        .await
        .expect("the agent must connect to the untrusted balancer");
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
                    version: 0,
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

    assert!(
        CapturingLogger::records_for("paddler_agent::balancer_message_context")
            .iter()
            .any(|CapturedRecord { level, .. }| *level == Level::Error)
    );

    untrusted_balancer
        .accept_agent()
        .await
        .expect("the agent must reconnect after ending the connection");
}
