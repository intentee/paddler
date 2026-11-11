use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_messaging::management_socket::agent::notification::Notification as AgentJsonRpcNotification;
use paddler_messaging::management_socket::agent::response::Response as AgentJsonRpcResponse;
use paddler_tests::agent_answer_after_a_notification::agent_answer_after_a_notification;
use paddler_tests::constants::ANSWERED_REQUEST_ID;

#[tokio::test(flavor = "multi_thread")]
async fn agent_keeps_a_management_connection_after_a_stop_for_an_unknown_request() {
    assert_eq!(
        agent_answer_after_a_notification(AgentJsonRpcNotification::StopRespondingTo(
            "unknown-request".to_owned(),
        ))
        .await
        .expect("the agent must answer after the notification"),
        ResponseEnvelope {
            generated_by: None,
            request_id: ANSWERED_REQUEST_ID.to_owned(),
            response: AgentJsonRpcResponse::ModelMetadata(None),
        }
    );
}
