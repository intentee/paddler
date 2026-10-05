use serde_json::json;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;

use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::raw_agent_socket::RawAgentSocket;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_closes_an_agent_socket_with_a_valid_close_frame_after_an_unknown_token_result() {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");
    let mut agent_socket =
        RawAgentSocket::connect(cluster.balancer.addresses.management, "newer-agent")
            .await
            .expect("the connection must be established");

    agent_socket
        .send(Message::text(
            json!({
                "Response": {
                    "generated_by": null,
                    "request_id": "request-answered-by-a-newer-agent",
                    "response": {
                        "GeneratedToken": {
                            "TokenResultFromANewerAgent": "piece"
                        }
                    }
                }
            })
            .to_string(),
        ))
        .await
        .expect("the message must be sent");

    assert_eq!(
        agent_socket
            .next_close_frame()
            .await
            .expect("the balancer must close the socket with a valid close frame")
            .map(|close_frame| close_frame.code),
        Some(CloseCode::Invalid)
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
