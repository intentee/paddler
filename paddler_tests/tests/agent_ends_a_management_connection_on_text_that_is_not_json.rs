use tokio_tungstenite::tungstenite::Message;

use paddler_tests::agent_hang_up_on_a_frame::agent_hang_up_on_a_frame;

#[tokio::test(flavor = "multi_thread")]
async fn agent_ends_a_management_connection_on_text_that_is_not_json() {
    agent_hang_up_on_a_frame(Message::text("not json"))
        .await
        .expect("the agent must hang up on the frame and reconnect");
}
