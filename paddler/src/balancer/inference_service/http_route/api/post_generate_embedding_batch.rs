use actix_web::Error;
use actix_web::HttpResponse;
use actix_web::Responder;
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
use log::error;
use nanoid::nanoid;
use paddler_types::embedding_result::EmbeddingResult;
use paddler_types::inference_client::Message as OutgoingMessage;
use paddler_types::inference_client::Response as OutgoingResponse;
use paddler_types::jsonrpc::Error as JsonRpcError;
use paddler_types::jsonrpc::ErrorEnvelope;
use paddler_types::jsonrpc::ResponseEnvelope;
use paddler_types::request_params::GenerateEmbeddingBatchParams;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_util::sync::CancellationToken;

use crate::balancer::chunk_forwarding_session_controller::ChunkForwardingSessionController;
use crate::balancer::chunk_forwarding_session_controller::identity_transformer::IdentityTransformer;
use crate::balancer::chunk_forwarding_session_controller::transform_result::TransformResult;
use crate::balancer::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;
use crate::balancer::inference_service::app_data::AppData;
use crate::balancer::request_from_agent::request_from_agent;
use crate::controls_session::ControlsSession as _;

const CHARACTERS_PER_TOKEN_APPROXIMATELY: usize = 3;

#[derive(Clone)]
struct EmbeddingChunkBodyTransformer;

#[async_trait]
impl TransformsOutgoingMessage for EmbeddingChunkBodyTransformer {
    async fn transform(&self, message: OutgoingMessage) -> Result<TransformResult> {
        if let OutgoingMessage::Response(ResponseEnvelope {
            response: OutgoingResponse::Embedding(EmbeddingResult::Done),
            ..
        }) = &message
        {
            return Ok(TransformResult::Discard);
        }

        let serialized = serde_json::to_string(&message)?;

        Ok(TransformResult::Chunk(serialized))
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
    let balancer_applicable_state_holder = app_data.balancer_applicable_state_holder.clone();
    let Some(agent_desired_state) = balancer_applicable_state_holder.get_agent_desired_state()
    else {
        return Err(ErrorServiceUnavailable(
            "Balancer applicable state is not yet set",
        ));
    };

    if !agent_desired_state.inference_parameters.enable_embeddings {
        return Err(ErrorNotImplemented(
            "Embedding generation is not enabled in the inference parameters",
        ));
    }

    let connection_close = CancellationToken::new();
    let (chunk_tx, chunk_rx) = mpsc::unbounded_channel();

    let mut chunk_tasks: JoinSet<()> = JoinSet::new();

    for batch in params.chunk_by_input_size(
        agent_desired_state.inference_parameters.batch_n_tokens
            * CHARACTERS_PER_TOKEN_APPROXIMATELY,
    ) {
        let buffered_request_manager_clone = app_data.buffered_request_manager.clone();
        let chunk_tx_clone = chunk_tx.clone();
        let connection_close_clone = connection_close.clone();
        let inference_service_configuration_clone =
            app_data.inference_service_configuration.clone();

        chunk_tasks.spawn(async move {
            let request_id: String = nanoid!();
            let mut session_controller = ChunkForwardingSessionController::new(
                chunk_tx_clone,
                EmbeddingChunkBodyTransformer,
            );

            if let Err(err) = request_from_agent(
                buffered_request_manager_clone,
                connection_close_clone,
                inference_service_configuration_clone,
                batch,
                request_id.clone(),
                session_controller.clone(),
            )
            .await
            {
                error!("Failed to handle request: {err}");
                session_controller
                    .send_response_safe(OutgoingMessage::Error(ErrorEnvelope {
                        request_id: request_id.clone(),
                        error: JsonRpcError {
                            code: 500,
                            description: format!("Request {request_id} failed: {err}"),
                        },
                    }))
                    .await;
            }
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
                request_id: final_request_id,
                response: OutgoingResponse::Embedding(EmbeddingResult::Done),
            }))
            .await;
    });

    drop(chunk_tx);

    let stream = UnboundedReceiverStream::new(chunk_rx).filter_map(|transform_result| async move {
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
