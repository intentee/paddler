use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use paddler::balancer::chunk_forwarding_session_controller::ChunkForwardingSessionController;
use paddler::balancer::chunk_forwarding_session_controller::identity_transformer::IdentityTransformer;
use paddler::balancer::chunk_forwarding_session_controller::transform_result::TransformResult;
use paddler::balancer::embedding_sender_collection::EmbeddingSenderCollection;
use paddler::balancer::inference_client::Message as OutgoingMessage;
use paddler::balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use paddler::balancer::manages_senders::ManagesSenders as _;
use paddler::balancer::manages_senders_controller::ManagesSendersController;
use paddler::balancer::request_from_agent::forward_responses_stream;
use paddler::jsonrpc::ErrorEnvelope;
use paddler_tests::make_agent_controller_without_remote_agent::make_agent_controller_without_remote_agent;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn forward_responses_stream_emits_error_envelope_when_response_channel_closes_before_terminator()
-> Result<()> {
    let agent_controller = Arc::new(make_agent_controller_without_remote_agent("test-agent"));
    let request_id = "test-request".to_owned();
    let receive_response_controller: ManagesSendersController<EmbeddingSenderCollection> =
        ManagesSendersController::from_request_id(
            request_id.clone(),
            agent_controller.embedding_sender_collection.clone(),
        )?;

    let (chunk_tx, mut chunk_rx) = mpsc::unbounded_channel::<TransformResult>();
    let session_controller =
        ChunkForwardingSessionController::new(chunk_tx, IdentityTransformer::new());

    let connection_close = CancellationToken::new();
    let configuration = InferenceServiceConfiguration {
        addr: SocketAddr::from(([127, 0, 0, 1], 0)),
        cors_allowed_hosts: Vec::new(),
        inference_item_timeout: Duration::from_secs(30),
    };

    let agent_controller_clone = agent_controller.clone();
    let request_id_clone = request_id.clone();
    let forward_handle: tokio::task::JoinHandle<Result<()>> = tokio::spawn(async move {
        forward_responses_stream::<_, EmbeddingSenderCollection>(
            agent_controller_clone,
            connection_close,
            configuration,
            receive_response_controller,
            request_id_clone,
            session_controller,
        )
        .await
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    agent_controller
        .embedding_sender_collection
        .deregister_sender(request_id.clone())?;

    tokio::time::timeout(Duration::from_secs(5), forward_handle)
        .await
        .context("forward_responses_stream did not complete in time")?
        .context("forward_responses_stream task panicked")?
        .context("forward_responses_stream returned an error")?;

    let chunk = chunk_rx
        .recv()
        .await
        .ok_or_else(|| anyhow!("expected an error envelope to be forwarded to the client"))?;

    let serialized = match chunk {
        TransformResult::Chunk(serialized) | TransformResult::Error(serialized) => serialized,
        TransformResult::Discard => {
            return Err(anyhow!("expected a Chunk transform result, got Discard"));
        }
    };

    let envelope: OutgoingMessage =
        serde_json::from_str(&serialized).context("failed to parse forwarded envelope as JSON")?;

    let OutgoingMessage::Error(ErrorEnvelope { error, .. }) = envelope else {
        return Err(anyhow!("expected an Error envelope"));
    };

    assert_eq!(
        error.code, 502,
        "expected 502 error code for premature channel close, got {}",
        error.code
    );

    Ok(())
}
