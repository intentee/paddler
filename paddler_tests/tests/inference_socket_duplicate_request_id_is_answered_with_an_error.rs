use std::num::NonZeroU32;

use std::num::NonZeroUsize;

use futures_util::StreamExt as _;
use paddler_client::inference_socket::pool::Pool;
use paddler_messaging::inference_client::message::Message as InferenceClientMessage;
use paddler_messaging::inference_server::message::Message as InferenceServerMessage;
use paddler_messaging::inference_server::request::Request as InferenceServerRequest;
use paddler_messaging::jsonrpc::request_envelope::RequestEnvelope;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;
use tokio_util::sync::CancellationToken;

const MAX_TOKENS: NonZeroU32 = NonZeroU32::new(16).unwrap();
const DUPLICATE_REQUEST_ID: &str = "duplicate-request-id";

fn raw_prompt_message(request_id: &str) -> InferenceServerMessage<ValidatedParametersSchema> {
    InferenceServerMessage::Request(RequestEnvelope {
        id: request_id.to_owned(),
        request: InferenceServerRequest::ContinueFromRawPrompt(ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: MAX_TOKENS,
            raw_prompt: "The capital of France is".to_owned(),
        }),
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn inference_socket_duplicate_request_id_is_answered_with_an_error() {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::without_request_expiry()
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

    let _first_request = pool
        .send_request(
            CancellationToken::new(),
            DUPLICATE_REQUEST_ID.to_owned(),
            raw_prompt_message(DUPLICATE_REQUEST_ID),
        )
        .await
        .expect("the first request must be sent");
    let message = pool
        .send_request(
            CancellationToken::new(),
            DUPLICATE_REQUEST_ID.to_owned(),
            raw_prompt_message(DUPLICATE_REQUEST_ID),
        )
        .await
        .expect("the duplicate request must be sent")
        .next()
        .await
        .expect(
            "a duplicate request id must be answered instead of leaving the client waiting forever",
        )
        .expect("the answer to the duplicate request must be readable");

    assert!(matches!(
        message,
        InferenceClientMessage::Error(envelope) if envelope.error.code == 400
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
