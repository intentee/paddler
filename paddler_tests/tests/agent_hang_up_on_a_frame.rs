use tokio_tungstenite::tungstenite::Message;

use paddler_test_cluster_harness::untrusted_balancer_socket_server::UntrustedBalancerSocketServer;

use crate::agent_runner_connected_to::agent_runner_connected_to;

pub async fn agent_hang_up_on_a_frame(frame: Message) {
    let untrusted_balancer = UntrustedBalancerSocketServer::bind()
        .await
        .expect("the untrusted balancer must bind");
    let _agent_runner = agent_runner_connected_to(untrusted_balancer.management_addr, 1);
    let mut first_connection = untrusted_balancer
        .accept_agent()
        .await
        .expect("the agent must connect");

    first_connection
        .send(frame)
        .await
        .expect("the frame must be sent");
    first_connection.wait_for_the_agent_to_hang_up().await;

    untrusted_balancer
        .accept_agent()
        .await
        .expect("the agent must reconnect after ending the connection");
}
