use std::collections::BTreeMap;

use tokio::join;
use tokio_util::sync::CancellationToken;

use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_messaging::management_socket::agent::response::Response as AgentJsonRpcResponse;
use paddler_messaging::model_metadata::ModelMetadata;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::raw_agent_socket::RawAgentSocket;
use paddler_tests::start_cluster::start_cluster;

const RAW_AGENT_ID: &str = "confused-agent";

#[tokio::test(flavor = "multi_thread")]
async fn balancer_keeps_an_agent_connected_after_it_answers_an_unknown_request() {
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
    let answered_metadata =
        BTreeMap::from([("general.architecture".to_owned(), "qwen3".to_owned())]);

    raw_agent_socket
        .register()
        .await
        .expect("the raw agent must register");
    raw_agent_socket
        .send_response(ResponseEnvelope {
            generated_by: None,
            request_id: "a-request-the-balancer-never-sent".to_owned(),
            response: AgentJsonRpcResponse::ModelMetadata(None),
        })
        .await
        .expect("the raw agent must send its answer to an unknown request");

    let (metadata_result, ()) = join!(
        cluster
            .client_management
            .get_model_metadata(CancellationToken::new(), RAW_AGENT_ID),
        async {
            let request_envelope = raw_agent_socket
                .next_request()
                .await
                .expect("the balancer must forward the metadata request to the agent");

            raw_agent_socket
                .send_response(ResponseEnvelope {
                    generated_by: None,
                    request_id: request_envelope.id,
                    response: AgentJsonRpcResponse::ModelMetadata(Some(ModelMetadata {
                        metadata: answered_metadata.clone(),
                    })),
                })
                .await
                .expect("the raw agent must answer the metadata request");
        }
    );

    assert_eq!(
        metadata_result
            .expect("the balancer must relay the agent's answer")
            .map(|model_metadata| model_metadata.metadata),
        Some(answered_metadata)
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
