use std::num::NonZeroUsize;

use anyhow::Context as _;
use anyhow::Result;
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

const DUPLICATE_REQUEST_ID: &str = "duplicate-request-id";

fn raw_prompt_message(request_id: &str) -> InferenceServerMessage<ValidatedParametersSchema> {
    InferenceServerMessage::Request(RequestEnvelope {
        id: request_id.to_owned(),
        request: InferenceServerRequest::ContinueFromRawPrompt(ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 16,
            raw_prompt: "The capital of France is".to_owned(),
        }),
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn inference_socket_duplicate_request_id_is_answered_with_an_error() -> Result<()> {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::without_request_expiry()
    })
    .await?;

    let pool = Pool::new(
        cluster.balancer.addresses.inference_base_url()?,
        NonZeroUsize::MIN,
    );

    let _first_request = pool
        .send_request(
            CancellationToken::new(),
            DUPLICATE_REQUEST_ID.to_owned(),
            raw_prompt_message(DUPLICATE_REQUEST_ID),
        )
        .await
        .map_err(anyhow::Error::new)?;

    let mut duplicate_request = pool
        .send_request(
            CancellationToken::new(),
            DUPLICATE_REQUEST_ID.to_owned(),
            raw_prompt_message(DUPLICATE_REQUEST_ID),
        )
        .await
        .map_err(anyhow::Error::new)?;

    let message = duplicate_request
        .next()
        .await
        .context(
            "a duplicate request id must be answered instead of leaving the client waiting forever",
        )?
        .map_err(anyhow::Error::new)?;

    match message {
        InferenceClientMessage::Error(envelope) => {
            assert_eq!(envelope.error.code, 400);
        }
        InferenceClientMessage::Notification(_) | InferenceClientMessage::Response(_) => {
            anyhow::bail!("a duplicate request id must be answered with an error, got {message:?}");
        }
    }

    cluster.shutdown().await?;

    Ok(())
}
