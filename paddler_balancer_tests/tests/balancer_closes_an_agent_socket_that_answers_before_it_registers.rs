use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;

use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_cluster_harness::untrusted_management_agent::UntrustedManagementAgent;
use paddler_messaging::management_socket::agent::response::Response as AgentJsonRpcResponse;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_closes_an_agent_socket_that_answers_before_it_registers() {
    let cluster = Cluster::start_in_process(ClusterParams::without_agents())
        .await
        .expect("a cluster without agents must start");
    let mut unregistered_agent =
        UntrustedManagementAgent::connect(cluster.balancer.addresses.management, "unregistered")
            .await
            .expect("the agent must reach the agent socket");

    unregistered_agent
        .respond(
            "never-dispatched".to_owned(),
            AgentJsonRpcResponse::ModelMetadata(None),
        )
        .await
        .expect("the response must be sent");

    assert_eq!(
        unregistered_agent
            .socket_client
            .next_close_frame()
            .await
            .expect("the balancer must close the socket")
            .map(|close_frame| (close_frame.code, close_frame.reason.to_string())),
        Some((
            CloseCode::Policy,
            "Agent sent a response before registering".to_owned()
        ))
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
