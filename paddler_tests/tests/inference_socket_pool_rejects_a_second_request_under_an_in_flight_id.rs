use std::num::NonZeroU32;
use std::num::NonZeroUsize;

use tokio_util::sync::CancellationToken;

use paddler_client::error::Error as ClientError;
use paddler_client::inference_socket::pool::Pool;
use paddler_messaging::inference_server::message::Message as InferenceServerMessage;
use paddler_messaging::inference_server::request::Request as InferenceServerRequest;
use paddler_messaging::jsonrpc::request_envelope::RequestEnvelope;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

const IN_FLIGHT_REQUEST_ID: &str = "in-flight-request-id";

fn raw_prompt_message() -> InferenceServerMessage<ValidatedParametersSchema> {
    InferenceServerMessage::Request(RequestEnvelope {
        id: IN_FLIGHT_REQUEST_ID.to_owned(),
        request: InferenceServerRequest::ContinueFromRawPrompt(ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: NonZeroU32::MIN,
            raw_prompt: "The capital of France is".to_owned(),
        }),
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn inference_socket_pool_rejects_a_second_request_under_an_in_flight_id() {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a cluster without agents must start");
    let pool = Pool::new(
        cluster
            .balancer
            .inference_base_url()
            .expect("the inference service must have a base URL"),
        NonZeroUsize::MIN,
    );

    let _in_flight_request = pool
        .send_request(
            CancellationToken::new(),
            IN_FLIGHT_REQUEST_ID.to_owned(),
            raw_prompt_message(),
        )
        .await
        .expect("the first request must be sent");
    let rejection = pool
        .send_request(
            CancellationToken::new(),
            IN_FLIGHT_REQUEST_ID.to_owned(),
            raw_prompt_message(),
        )
        .await;

    assert!(matches!(
        rejection,
        Err(ClientError::InferenceRequestIdInFlight { request_id }) if request_id == IN_FLIGHT_REQUEST_ID
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
