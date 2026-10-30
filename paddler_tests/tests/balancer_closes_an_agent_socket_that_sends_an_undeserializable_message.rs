use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;

use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

use crate::untrusted_management_agent::UntrustedManagementAgent;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_closes_an_agent_socket_that_sends_an_undeserializable_message() {
    let cluster = start_cluster(ClusterParams::without_agents())
        .await
        .expect("the cluster must start");
    let mut undeserializable_agent = UntrustedManagementAgent::connect(
        cluster.balancer.addresses.management,
        "undeserializable-agent",
    )
    .await
    .expect("the connection must be established");

    undeserializable_agent
        .socket_client
        .send(Message::text("not json"))
        .await
        .expect("the message must be sent");

    assert_eq!(
        undeserializable_agent
            .socket_client
            .next_close_frame()
            .await
            .expect("the balancer must close the socket")
            .map(|close_frame| close_frame.code),
        Some(CloseCode::Invalid)
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
