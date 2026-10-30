use std::sync::Arc;

use actix_web::Error;
use actix_web::HttpResponse;
use actix_web::Responder;
use actix_web::http::header;
use actix_web::rt;
use actix_web::web;
use actix_web::web::post;
use anyhow::Result;
use futures::stream::StreamExt;
use nanoid::nanoid;
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

use crate::agent_relay::AgentRelay;
use crate::awaiting_agent_relay::AwaitingAgentRelay;
use crate::cancellation_token_stream_guard::CancellationTokenStreamGuard;
use crate::chunk_forwarding_session_controller::ChunkForwardingSessionController;
use crate::chunk_forwarding_session_controller::identity_transformer::IdentityTransformer;
use crate::chunk_forwarding_session_controller::transform_result::TransformResult;
use crate::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;
use crate::controls_session::ControlsSession as _;
use crate::dispatchable_request::DispatchableRequest;
use crate::distinct_embedding_documents::DistinctEmbeddingDocuments;
use crate::embedding_chunk_dispatch::EmbeddingChunkDispatch;
use crate::embedding_document_duplicates::EmbeddingDocumentDuplicates;
use crate::forward_agent_relay::forward_agent_relay;
use crate::inference_service::app_data::AppData;
use crate::outgoing_message_relay_receiver::OutgoingMessageRelayReceiver;

async fn respond(
    app_data: web::Data<AppData>,
    params: web::Json<GenerateEmbeddingBatchParams>,
) -> Result<impl Responder, Error> {
    let embedding_batch_size = app_data
        .balancer_applicable_state_holder
        .get_balancer_applicable_state()
        .balancer_desired_state
        .embeddings
        .embedding_batch_size;

    let connection_close = CancellationToken::new();
    let request_id: String = nanoid!();
    let (chunk_tx, chunk_rx) = mpsc::unbounded_channel();

    let mut chunk_tasks: JoinSet<()> = JoinSet::new();

    let DistinctEmbeddingDocuments {
        duplicates,
        params: distinct_params,
    } = DistinctEmbeddingDocuments::from(params.into_inner());
    let embedding_document_duplicates = Arc::new(duplicates);
    let batches = distinct_params.chunk_evenly_with_cap(
        app_data.agent_controller_pool.agent_count(),
        embedding_batch_size,
    );

    let chunk_dispatches: Vec<EmbeddingChunkDispatch> = batches
        .into_iter()
        .map(|batch| {
            let dispatchable_batch = DispatchableRequest::from(batch);

            match app_data
                .buffered_request_manager
                .take_available_agent(dispatchable_batch.dispatch_requirement)
            {
                Some(dispatched_agent) => EmbeddingChunkDispatch::Claimed {
                    batch: dispatchable_batch.params,
                    dispatched_agent,
                },
                None => EmbeddingChunkDispatch::Buffered(dispatchable_batch),
            }
        })
        .collect();

    let inference_item_timeout = app_data
        .inference_service_configuration
        .inference_item_timeout;

    for chunk_dispatch in chunk_dispatches {
        let session_controller = ChunkForwardingSessionController::new(
            chunk_tx.clone(),
            EmbeddingChunkBodyTransformer {
                embedding_document_duplicates: embedding_document_duplicates.clone(),
            },
        );

        let agent_relay = match chunk_dispatch {
            EmbeddingChunkDispatch::Claimed {
                batch,
                dispatched_agent,
            } => AgentRelay::dispatched(
                dispatched_agent,
                batch,
                inference_item_timeout,
                app_data.shutdown.clone(),
            ),
            EmbeddingChunkDispatch::Buffered(dispatchable_batch) => {
                AgentRelay::awaiting(AwaitingAgentRelay {
                    buffered_request_manager: app_data.buffered_request_manager.clone(),
                    dispatchable_request: dispatchable_batch,
                    inference_item_timeout,
                    shutdown: app_data.shutdown.clone(),
                })
            }
        };

        chunk_tasks.spawn(forward_agent_relay(
            agent_relay,
            connection_close.clone(),
            OutgoingMessageRelayReceiver {
                request_id: request_id.clone(),
                session_controller,
            },
        ));
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

struct EmbeddingChunkBodyTransformer {
    embedding_document_duplicates: Arc<EmbeddingDocumentDuplicates>,
}

impl TransformsOutgoingMessage for EmbeddingChunkBodyTransformer {
    type Output = TransformResult;

    fn transform(&self, message: OutgoingMessage) -> Vec<TransformResult> {
        if let OutgoingMessage::Response(ResponseEnvelope {
            response: OutgoingResponse::Embedding(EmbeddingResult::Done),
            ..
        }) = &message
        {
            return vec![TransformResult::Discard];
        }

        self.embedding_document_duplicates
            .fan_out(message)
            .iter()
            .map(TransformResult::serialized_from)
            .collect()
    }
}

pub fn post_generate_embedding_batch(cfg: &mut web::ServiceConfig) {
    cfg.route(ApiPath::GENERATE_EMBEDDING_BATCH, post().to(respond));
}
