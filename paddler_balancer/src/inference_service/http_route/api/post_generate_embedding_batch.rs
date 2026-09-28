use actix_web::Error;
use actix_web::HttpResponse;
use actix_web::Responder;
use actix_web::error::ErrorInternalServerError;
use actix_web::error::ErrorNotImplemented;
use actix_web::error::ErrorServiceUnavailable;
use actix_web::http::header;
use actix_web::post;
use actix_web::rt;
use actix_web::web;
use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::StreamExt;
use nanoid::nanoid;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::inference_client::message::Message as OutgoingMessage;
use paddler_messaging::inference_client::response::Response as OutgoingResponse;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_messaging::request_params::generate_embedding_batch_params::chunk_evenly_with_cap_error::ChunkEvenlyWithCapError;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_util::sync::CancellationToken;

use crate::cancellation_token_stream_guard::CancellationTokenStreamGuard;
use crate::chunk_forwarding_session_controller::ChunkForwardingSessionController;
use crate::chunk_forwarding_session_controller::identity_transformer::IdentityTransformer;
use crate::chunk_forwarding_session_controller::transform_result::TransformResult;
use crate::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;
use crate::controls_session::ControlsSession as _;
use crate::inference_service::app_data::AppData;
use crate::request_from_agent::request_from_agent;

#[derive(Clone)]
struct EmbeddingChunkBodyTransformer;

#[async_trait]
impl TransformsOutgoingMessage for EmbeddingChunkBodyTransformer {
    type Output = TransformResult;

    async fn transform(&self, message: OutgoingMessage) -> Result<Vec<TransformResult>> {
        if let OutgoingMessage::Response(ResponseEnvelope {
            response: OutgoingResponse::Embedding(EmbeddingResult::Done),
            ..
        }) = &message
        {
            return Ok(vec![TransformResult::Discard]);
        }

        let serialized = serde_json::to_string(&message)?;

        Ok(vec![TransformResult::Chunk(serialized)])
    }
}

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}

#[post("/api/v1/generate_embedding_batch")]
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

    let agent_count = app_data.agent_controller_pool.agents.len();
    let embedding_batch_size = agent_desired_state
        .inference_parameters
        .embedding_batch_size;

    let connection_close = CancellationToken::new();
    let (chunk_tx, chunk_rx) = mpsc::unbounded_channel();

    let mut chunk_tasks: JoinSet<()> = JoinSet::new();

    let batches = match params
        .into_inner()
        .chunk_evenly_with_cap(agent_count, embedding_batch_size)
    {
        Ok(batches) => batches,
        Err(ChunkEvenlyWithCapError::ZeroAgentCount) => {
            return Err(ErrorServiceUnavailable("No agents are currently connected"));
        }
        Err(ChunkEvenlyWithCapError::ZeroMaxDocumentsPerChunk) => {
            return Err(ErrorInternalServerError(
                "embedding_batch_size is zero despite validation",
            ));
        }
    };

    for batch in batches {
        let buffered_request_manager_clone = app_data.buffered_request_manager.clone();
        let chunk_tx_clone = chunk_tx.clone();
        let connection_close_clone = connection_close.clone();
        let inference_service_configuration_clone =
            app_data.inference_service_configuration.clone();
        let shutdown_clone = app_data.shutdown.clone();

        chunk_tasks.spawn(async move {
            let request_id: String = nanoid!();
            let session_controller = ChunkForwardingSessionController::new(
                chunk_tx_clone,
                EmbeddingChunkBodyTransformer,
            );

            request_from_agent(
                buffered_request_manager_clone,
                connection_close_clone,
                inference_service_configuration_clone,
                batch,
                request_id,
                session_controller,
                shutdown_clone,
            )
            .await;
        });
    }

    let final_done_chunk_tx = chunk_tx.clone();

    rt::spawn(async move {
        while chunk_tasks.join_next().await.is_some() {}

        let final_request_id: String = nanoid!();
        let mut final_session =
            ChunkForwardingSessionController::new(final_done_chunk_tx, IdentityTransformer::new());

        final_session
            .send_response_safe(OutgoingMessage::Response(ResponseEnvelope {
                generated_by: None,
                request_id: final_request_id,
                response: OutgoingResponse::Embedding(EmbeddingResult::Done),
            }))
            .await;
    });

    drop(chunk_tx);

    let stream =
        CancellationTokenStreamGuard::new(connection_close, UnboundedReceiverStream::new(chunk_rx))
            .filter_map(|transform_result| async move {
                match transform_result {
                    TransformResult::Chunk(content) | TransformResult::Error(content) => {
                        Some(Ok::<_, Error>(Bytes::from(format!("{content}\n"))))
                    }
                    TransformResult::Discard => None,
                }
            });

    Ok(HttpResponse::Ok()
        .insert_header(header::ContentType::json())
        .insert_header((header::CACHE_CONTROL, "no-cache"))
        .streaming(stream))
}
