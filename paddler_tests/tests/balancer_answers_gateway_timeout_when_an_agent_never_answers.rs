use std::mem::discriminant;

use http::StatusCode;
use tokio::join;
use tokio_util::sync::CancellationToken;

use paddler_client::error::Error as ClientError;
use paddler_messaging::management_socket::agent::request::Request as AgentJsonRpcRequest;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::raw_agent_socket::RawAgentSocket;
use paddler_tests::start_cluster::start_cluster;

const RAW_AGENT_ID: &str = "silent-agent";

#[tokio::test(flavor = "multi_thread")]
async fn balancer_answers_gateway_timeout_when_an_agent_never_answers() {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a balancer without agents must start");
    let mut raw_agent_socket =
        RawAgentSocket::connect(cluster.balancer.addresses.management, RAW_AGENT_ID)
            .await
            .expect("the raw agent must reach the agent socket");

    raw_agent_socket
        .register()
        .await
        .expect("the raw agent must register");

    let (metadata_result, unanswered_request) = join!(
        cluster
            .client_management
            .get_model_metadata(CancellationToken::new(), RAW_AGENT_ID),
        raw_agent_socket.next_request()
    );

    assert_eq!(
        discriminant(
            &unanswered_request
                .expect("the balancer must forward the metadata request to the agent")
                .request
        ),
        discriminant(&AgentJsonRpcRequest::GetModelMetadata)
    );
    assert!(matches!(
        metadata_result,
        Err(ClientError::UnexpectedResponseStatus { status, .. }) if status == StatusCode::GATEWAY_TIMEOUT
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
