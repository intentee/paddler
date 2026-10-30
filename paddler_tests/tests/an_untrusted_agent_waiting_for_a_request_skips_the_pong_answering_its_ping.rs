use std::mem::discriminant;

use tokio::spawn;
use tokio_tungstenite::tungstenite::Bytes;
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;

use paddler_messaging::management_socket::agent::request::Request as AgentJsonRpcRequest;
use paddler_messaging::management_socket::agent::response::Response as AgentJsonRpcResponse;
use paddler_messaging::management_socket::balancer::notification_params::register_agent_params::RegisterAgentParams;
use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

use crate::untrusted_management_agent::UntrustedManagementAgent;

const PINGING_AGENT_ID: &str = "pinging";

#[tokio::test(flavor = "multi_thread")]
async fn an_untrusted_agent_waiting_for_a_request_skips_the_pong_answering_its_ping() {
    let mut cluster = start_cluster(ClusterParams::without_agents())
        .await
        .expect("a cluster without agents must start");
    let mut pinging_agent =
        UntrustedManagementAgent::connect(cluster.balancer.addresses.management, PINGING_AGENT_ID)
            .await
            .expect("the pinging agent must reach the agent socket");

    pinging_agent
        .socket_client
        .send(Message::Ping(Bytes::new()))
        .await
        .expect("the ping must be sent");
    pinging_agent
        .register(RegisterAgentParams {
            name: Some(PINGING_AGENT_ID.to_owned()),
            slot_aggregated_status_snapshot: SlotAggregatedStatusSnapshot::default(),
        })
        .await
        .expect("the pinging agent must register");
    cluster
        .wait_for_agent_count(1)
        .await
        .expect("the balancer must register the pinging agent after answering its ping");

    let client_management = cluster.client_management.clone();
    let model_metadata_answer = spawn(async move {
        client_management
            .get_model_metadata(CancellationToken::new(), PINGING_AGENT_ID)
            .await
    });
    let model_metadata_request = pinging_agent
        .next_request()
        .await
        .expect("the balancer must ask the pinging agent for its model metadata");

    assert_eq!(
        discriminant(&model_metadata_request.request),
        discriminant(&AgentJsonRpcRequest::GetModelMetadata)
    );

    pinging_agent
        .respond(
            model_metadata_request.id,
            AgentJsonRpcResponse::ModelMetadata(None),
        )
        .await
        .expect("the model metadata answer must be sent");
    model_metadata_answer
        .await
        .expect("the model metadata request must finish")
        .expect("the balancer must relay the model metadata answer");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
