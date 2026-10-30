use serde_json::to_string;
use tokio_tungstenite::tungstenite::Message;

use paddler_messaging::jsonrpc::request_envelope::RequestEnvelope;
use paddler_messaging::management_socket::agent::message::Message as AgentJsonRpcMessage;
use paddler_messaging::management_socket::agent::request::Request as AgentJsonRpcRequest;
use paddler_tcp_buffer_limits::tcp_buffer_limits_file::TcpBufferLimitsFile;
use paddler_test_cluster_harness::untrusted_balancer_socket_server::UntrustedBalancerSocketServer;

use crate::agent_runner_connected_to::agent_runner_connected_to;

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_whose_balancer_stops_reading_reconnects() {
    let largest_send_buffer = TcpBufferLimitsFile::of_kernel_send_buffers()
        .largest_buffer()
        .expect("the kernel must report its TCP send buffer limits");
    let largest_receive_buffer = TcpBufferLimitsFile::of_kernel_receive_buffers()
        .largest_buffer()
        .expect("the kernel must report its TCP receive buffer limits");
    let untrusted_balancer = UntrustedBalancerSocketServer::bind()
        .await
        .expect("the untrusted balancer must bind");
    let _agent_runner = agent_runner_connected_to(untrusted_balancer.management_addr, 1);
    let mut unread_connection = untrusted_balancer
        .accept_agent()
        .await
        .expect("the agent must connect");

    for request_index in 0..=(largest_receive_buffer / largest_send_buffer + 1) {
        unread_connection
            .send(Message::text(
                to_string(&AgentJsonRpcMessage::Request(RequestEnvelope {
                    id: format!("{request_index}-{}", "x".repeat(largest_send_buffer)),
                    request: AgentJsonRpcRequest::GetModelMetadata,
                }))
                .expect("the agent request must serialize"),
            ))
            .await
            .expect("the agent must read every request whose answer it cannot deliver");
    }

    untrusted_balancer
        .accept_agent()
        .await
        .expect("the agent must reconnect once its balancer stops reading its answers");
}
