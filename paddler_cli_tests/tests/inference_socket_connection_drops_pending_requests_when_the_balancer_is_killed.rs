use std::num::NonZeroU32;
use std::num::NonZeroUsize;
use std::sync::Arc;

use serde_json::to_string;

use paddler_cli_tests::start_signalled_subprocess_cluster::start_signalled_subprocess_cluster;
use paddler_cli_tests::subprocess_cluster::SubprocessCluster;
use paddler_client::error::Error;
use paddler_client::inference_socket::cluster_inference_mode_broadcaster::ClusterInferenceModeBroadcaster;
use paddler_client::inference_socket::connection::Connection;
use paddler_messaging::inference_server::message::Message as InferenceServerMessage;
use paddler_messaging::inference_server::request::Request as InferenceServerRequest;
use paddler_messaging::jsonrpc::request_envelope::RequestEnvelope;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

const PENDING_REQUEST_ID: &str = "pending-request";

#[tokio::test(flavor = "multi_thread")]
async fn inference_socket_connection_drops_pending_requests_when_the_balancer_is_killed() {
    let SubprocessCluster {
        balancer_signals,
        mut cluster,
    } = start_signalled_subprocess_cluster(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        ClusterParams {
            agents: Vec::new(),
            wait_for_slots_ready: false,
            ..ClusterParams::default()
        },
    )
    .await
    .expect("a balancer without agents must start");
    let connection = Connection::connect(
        cluster
            .balancer
            .inference_base_url()
            .expect("the balancer must expose its inference address"),
        Arc::new(ClusterInferenceModeBroadcaster::new(NonZeroUsize::MIN)),
    )
    .await
    .expect("the inference socket must connect");
    let request: InferenceServerMessage<ValidatedParametersSchema> =
        InferenceServerMessage::Request(RequestEnvelope {
            id: PENDING_REQUEST_ID.to_owned(),
            request: InferenceServerRequest::ContinueFromRawPrompt(ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::MIN,
                raw_prompt: "wait for an agent".to_owned(),
            }),
        });
    let mut response_rx = connection
        .send(
            PENDING_REQUEST_ID.to_owned(),
            to_string(&request).expect("the request must serialize"),
        )
        .expect("an open connection must accept the request");

    cluster
        .wait_for_buffered_request_count(1)
        .await
        .expect("the request must wait in the buffer while no agent is available");
    balancer_signals
        .kill()
        .expect("the balancer must be killed while the request is pending");

    assert!(matches!(
        response_rx.recv().await,
        Some(Err(Error::ConnectionDropped { request_id })) if request_id == PENDING_REQUEST_ID
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
