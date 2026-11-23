use log::Level;
use serde_json::from_str;
use tokio_tungstenite::tungstenite::Message;

use paddler_capturing_logger::captured_record::CapturedRecord;
use paddler_capturing_logger::capturing_logger::CapturingLogger;
use paddler_messaging::management_socket::agent::message::Message as AgentJsonRpcMessage;
use paddler_tests::agent_hang_up_on_a_frame::agent_hang_up_on_a_frame;

#[tokio::test(flavor = "multi_thread")]
async fn agent_ends_a_management_connection_on_text_that_is_not_json() {
    CapturingLogger::install();

    agent_hang_up_on_a_frame(Message::text("not json"))
        .await
        .expect("the agent must hang up on the frame and reconnect");

    assert!(
        CapturingLogger::records_for("paddler_agent::balancer_message_context").contains(&CapturedRecord {
            level: Level::Error,
            message: format!(
                "Closing the management connection after an undeserializable message not json: {}",
                from_str::<AgentJsonRpcMessage>("not json")
                    .err()
                    .expect("the text must not be a management message")
            ),
            target: "paddler_agent::balancer_message_context".to_owned(),
        })
    );
}
