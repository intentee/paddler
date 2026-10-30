use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::protocol::frame::Frame;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::frame::coding::Data;
use tokio_tungstenite::tungstenite::protocol::frame::coding::OpCode;

use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

use crate::untrusted_management_agent::UntrustedManagementAgent;

const FIRST_RESERVED_DATA_OPCODE: u8 = 3;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_closes_an_agent_socket_that_sends_a_reserved_opcode_frame_as_a_protocol_violation()
 {
    let cluster = start_cluster(ClusterParams::without_agents())
        .await
        .expect("the cluster must start");
    let mut protocol_violating_agent = UntrustedManagementAgent::connect(
        cluster.balancer.addresses.management,
        "protocol-violating-agent",
    )
    .await
    .expect("the connection must be established");

    protocol_violating_agent
        .socket_client
        .send(Message::Frame(Frame::message(
            "payload",
            OpCode::Data(Data::Reserved(FIRST_RESERVED_DATA_OPCODE)),
            true,
        )))
        .await
        .expect("the frame must be sent");

    assert_eq!(
        protocol_violating_agent
            .socket_client
            .next_close_frame()
            .await
            .expect("the balancer must close the socket with a valid close frame")
            .map(|close_frame| close_frame.code),
        Some(CloseCode::Protocol)
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
