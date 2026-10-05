use std::mem::discriminant;

use http::StatusCode;
use tokio::join;
use tokio_util::sync::CancellationToken;

use paddler_client::error::Error as ClientError;
use paddler_messaging::management_socket::agent::request::Request as AgentJsonRpcRequest;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::raw_agent_socket::RawAgentSocket;
use paddler_tests::start_cluster::start_cluster;

const RAW_AGENT_ID: &str = "disconnecting-agent";

#[tokio::test(flavor = "multi_thread")]
async fn balancer_answers_bad_gateway_when_an_agent_disconnects_before_answering() {
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

    let (metadata_result, ()) = join!(
        cluster
            .client_management
            .get_model_metadata(CancellationToken::new(), RAW_AGENT_ID),
        async move {
            let request_envelope = raw_agent_socket
                .next_request()
                .await
                .expect("the balancer must forward the metadata request to the agent");

            assert_eq!(
                discriminant(&request_envelope.request),
                discriminant(&AgentJsonRpcRequest::GetModelMetadata)
            );

            drop(raw_agent_socket);
        }
    );

    assert!(matches!(
        metadata_result,
        Err(ClientError::UnexpectedResponseStatus { status, .. }) if status == StatusCode::BAD_GATEWAY
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
