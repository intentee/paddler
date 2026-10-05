use std::num::NonZeroUsize;

use actix_web::Error;
use actix_web::HttpResponse;
use actix_web::Responder;
use actix_web::error::ErrorNotImplemented;
use actix_web::error::ErrorServiceUnavailable;
use actix_web::http::header;
use actix_web::rt;
use actix_web::web;
use actix_web::web::post;
use anyhow::Result;
use async_trait::async_trait;
use futures::stream::StreamExt;
use nanoid::nanoid;
use serde_json::to_string;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_util::sync::CancellationToken;

use paddler_messaging::api_path::ApiPath;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::inference_client::message::Message as OutgoingMessage;
use paddler_messaging::inference_client::response::Response as OutgoingResponse;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;

use crate::agent_relay_error::AgentRelayError;
use crate::cancellation_token_stream_guard::CancellationTokenStreamGuard;
use crate::chunk_forwarding_session_controller::ChunkForwardingSessionController;
use crate::chunk_forwarding_session_controller::identity_transformer::IdentityTransformer;
use crate::chunk_forwarding_session_controller::transform_result::TransformResult;
use crate::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;
use crate::controls_session::ControlsSession as _;
use crate::embedding_chunk_dispatch::EmbeddingChunkDispatch;
use crate::inference_service::app_data::AppData;
use crate::request_from_agent::request_from_agent;
use crate::request_from_dispatched_agent::request_from_dispatched_agent;

async fn respond(
    app_data: web::Data<AppData>,
    params: web::Json<GenerateEmbeddingBatchParams>,
) -> Result<impl Responder, Error> {
    let agent_desired_state = app_data
        .balancer_applicable_state_holder
        .get_agent_desired_state();

    if !agent_desired_state.inference_parameters.enable_embeddings {
        return Err(ErrorNotImplemented(
            "Embedding generation is not enabled in the inference parameters",
        ));
    }

    let Some(agent_count) = NonZeroUsize::new(app_data.agent_controller_pool.agents.len()) else {
        return Err(ErrorServiceUnavailable("No agents are currently connected"));
    };
    let embedding_batch_size = agent_desired_state
        .inference_parameters
        .embedding_batch_size;

    let connection_close = CancellationToken::new();
    let request_id: String = nanoid!();
    let (chunk_tx, chunk_rx) = mpsc::unbounded_channel();

    let mut chunk_tasks: JoinSet<()> = JoinSet::new();

    let batches = params
        .into_inner()
        .chunk_evenly_with_cap(agent_count, embedding_batch_size);

    let chunk_dispatches: Vec<EmbeddingChunkDispatch> = batches
        .into_iter()
        .map(
            |batch| match app_data.buffered_request_manager.take_available_agent() {
                Some(dispatched_agent) => EmbeddingChunkDispatch::Claimed {
                    batch,
                    dispatched_agent,
                },
                None => EmbeddingChunkDispatch::Buffered { batch },
            },
        )
        .collect();

    for chunk_dispatch in chunk_dispatches {
        let session_controller =
            ChunkForwardingSessionController::new(chunk_tx.clone(), EmbeddingChunkBodyTransformer);

        match chunk_dispatch {
            EmbeddingChunkDispatch::Claimed {
                batch,
                dispatched_agent,
            } => chunk_tasks.spawn(request_from_dispatched_agent(
                dispatched_agent,
                connection_close.clone(),
                app_data.inference_service_configuration.clone(),
                batch,
                request_id.clone(),
                session_controller,
                app_data.shutdown.clone(),
            )),
            EmbeddingChunkDispatch::Buffered { batch } => chunk_tasks.spawn(request_from_agent(
                app_data.buffered_request_manager.clone(),
                connection_close.clone(),
                app_data.inference_service_configuration.clone(),
                batch,
                request_id.clone(),
                session_controller,
                app_data.shutdown.clone(),
            )),
        };
    }

    let final_done_chunk_tx = chunk_tx.clone();

    rt::spawn(async move {
        while chunk_tasks.join_next().await.is_some() {}

        let mut final_session =
            ChunkForwardingSessionController::new(final_done_chunk_tx, IdentityTransformer::new());

        final_session
            .send_response_safe(OutgoingMessage::Response(ResponseEnvelope {
                generated_by: None,
                request_id,
                response: OutgoingResponse::Embedding(EmbeddingResult::Done),
            }))
            .await;
    });

    drop(chunk_tx);

    let stream =
        CancellationTokenStreamGuard::new(connection_close, UnboundedReceiverStream::new(chunk_rx))
            .filter_map(|transform_result: TransformResult| async move {
                transform_result.into_ndjson_line().map(Ok::<_, Error>)
            });

    Ok(HttpResponse::Ok()
        .insert_header(header::ContentType::json())
        .insert_header((header::CACHE_CONTROL, "no-cache"))
        .streaming(stream))
}

#[derive(Clone)]
struct EmbeddingChunkBodyTransformer;

#[async_trait]
impl TransformsOutgoingMessage for EmbeddingChunkBodyTransformer {
    type Output = TransformResult;

    async fn transform(
        &self,
        message: OutgoingMessage,
    ) -> Result<Vec<TransformResult>, AgentRelayError> {
        if let OutgoingMessage::Response(ResponseEnvelope {
            response: OutgoingResponse::Embedding(EmbeddingResult::Done),
            ..
        }) = &message
        {
            return Ok(vec![TransformResult::Discard]);
        }

        to_string(&message)
            .map(|serialized| vec![TransformResult::Chunk(serialized)])
            .map_err(AgentRelayError::MessageUnserializable)
    }
}

pub fn post_generate_embedding_batch(cfg: &mut web::ServiceConfig) {
    cfg.route(ApiPath::GENERATE_EMBEDDING_BATCH, post().to(respond));
}
