use tokio_tungstenite::tungstenite::Message;

use crate::agent_hang_up_on_a_frame::agent_hang_up_on_a_frame;

#[tokio::test(flavor = "multi_thread")]
async fn agent_ends_a_management_connection_on_a_binary_frame() {
    agent_hang_up_on_a_frame(Message::binary(vec![0_u8, 1])).await;
}
