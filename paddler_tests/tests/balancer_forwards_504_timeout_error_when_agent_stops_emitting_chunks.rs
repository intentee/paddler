use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use paddler_balancer::chunk_forwarding_session_controller::ChunkForwardingSessionController;
use paddler_balancer::chunk_forwarding_session_controller::identity_transformer::IdentityTransformer;
use paddler_balancer::chunk_forwarding_session_controller::transform_result::TransformResult;
use paddler_balancer::embedding_sender_collection::EmbeddingSenderCollection;
use paddler_balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use paddler_balancer::manages_senders_controller::ManagesSendersController;
use paddler_balancer::request_from_agent::forward_responses_stream;
use paddler_messaging::inference_client::message::Message as OutgoingMessage;
use paddler_messaging::jsonrpc::error_envelope::ErrorEnvelope;
use paddler_tests::make_agent_controller_without_remote_agent::make_agent_controller_without_remote_agent;
use paddler_tests::make_dispatched_agent_without_remote_agent::make_dispatched_agent_without_remote_agent;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_forwards_504_timeout_error_when_agent_stops_emitting_chunks() -> Result<()> {
    let agent_controller = Arc::new(make_agent_controller_without_remote_agent("test-agent"));
    let request_id = "timeout-test-request".to_owned();
    let receive_response_controller: ManagesSendersController<EmbeddingSenderCollection> =
        ManagesSendersController::from_request_id(
            request_id.clone(),
            agent_controller.embedding_sender_collection.clone(),
        )?;

    let (chunk_tx, mut chunk_rx) = mpsc::unbounded_channel::<TransformResult>();
    let session_controller =
        ChunkForwardingSessionController::new(chunk_tx, IdentityTransformer::new());

    let connection_close = CancellationToken::new();
    let inference_item_timeout = Duration::from_millis(150);
    let configuration = InferenceServiceConfiguration {
        addr: SocketAddr::from(([127, 0, 0, 1], 0)),
        cors_allowed_hosts: Vec::new(),
        inference_item_timeout,
    };

    let dispatched_agent = make_dispatched_agent_without_remote_agent(agent_controller.clone())?;
    let request_id_clone = request_id.clone();
    let forward_handle: tokio::task::JoinHandle<()> = tokio::spawn(async move {
        forward_responses_stream::<_, EmbeddingSenderCollection>(
            connection_close,
            dispatched_agent,
            configuration,
            receive_response_controller,
            request_id_clone,
            session_controller,
            CancellationToken::new(),
        )
        .await;
    });

    let forward_completed_within = inference_item_timeout * 10;
    tokio::time::timeout(forward_completed_within, forward_handle)
        .await
        .context("forward_responses_stream did not return within the 504-timeout budget")?
        .context("forward_responses_stream task panicked")?;

    let chunk = chunk_rx
        .recv()
        .await
        .ok_or_else(|| anyhow!("expected a 504 timeout envelope to be forwarded to the client"))?;

    let serialized = match chunk {
        TransformResult::Chunk(serialized) | TransformResult::Error(serialized) => serialized,
        TransformResult::Discard => {
            return Err(anyhow!(
                "expected a Chunk or Error transform result, got Discard"
            ));
        }
    };

    let envelope: OutgoingMessage =
        serde_json::from_str(&serialized).context("failed to parse forwarded envelope as JSON")?;

    let OutgoingMessage::Error(ErrorEnvelope { error, .. }) = envelope else {
        return Err(anyhow!("expected an Error envelope"));
    };

    assert_eq!(
        error.code, 504,
        "expected 504 error code for inference-item timeout, got {}",
        error.code,
    );

    Ok(())
}
