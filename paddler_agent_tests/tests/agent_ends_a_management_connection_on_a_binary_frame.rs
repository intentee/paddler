use log::Level;
use tokio_tungstenite::tungstenite::Message;

use paddler_agent_tests::agent_hang_up_on_a_frame::agent_hang_up_on_a_frame;
use paddler_capturing_logger::captured_record::CapturedRecord;
use paddler_capturing_logger::capturing_logger::CapturingLogger;

#[tokio::test(flavor = "multi_thread")]
async fn agent_ends_a_management_connection_on_a_binary_frame() {
    let capturing_logger = CapturingLogger::install();

    agent_hang_up_on_a_frame(Message::binary(vec![0_u8, 1]))
        .await
        .expect("the agent must hang up on the frame and reconnect");

    assert!(
        capturing_logger
            .records_for("paddler_agent::balancer_message_context")
            .iter()
            .any(|CapturedRecord { level, .. }| *level == Level::Error)
    );
}
