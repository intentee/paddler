use actix_web::Error;
use actix_web::HttpResponse;
use actix_web::Responder;
use actix_web::error::ErrorNotImplemented;
use actix_web::error::ErrorServiceUnavailable;
use actix_web::http::header;
use actix_web::post;
use actix_web::rt;
use actix_web::web;
use bytes::Bytes;
use futures::stream::StreamExt;
use log::error;
use nanoid::nanoid;
use paddler_types::inference_client::Message as OutgoingMessage;
use paddler_types::jsonrpc::Error as JsonRpcError;
use paddler_types::jsonrpc::ErrorEnvelope;
use paddler_types::request_params::GenerateEmbeddingBatchParams;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_util::sync::CancellationToken;

use crate::balancer::chunk_forwarding_session_controller::ChunkForwardingSessionController;
use crate::balancer::chunk_forwarding_session_controller::identity_transformer::IdentityTransformer;
use crate::balancer::chunk_forwarding_session_controller::transform_result::TransformResult;
use crate::balancer::inference_service::app_data::AppData;
use crate::balancer::request_from_agent::request_from_agent;
use crate::controls_session::ControlsSession as _;

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
    let buffered_request_manager = app_data.buffered_request_manager.clone();
    let inference_service_configuration = app_data.inference_service_configuration.clone();
    let batch = params.into_inner();

    rt::spawn(async move {
        let request_id: String = nanoid!();
        let mut session_controller =
            ChunkForwardingSessionController::new(chunk_tx, IdentityTransformer::new());

        if let Err(err) = request_from_agent(
            buffered_request_manager,
            connection_close,
            inference_service_configuration,
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

    let stream = UnboundedReceiverStream::new(chunk_rx).map(|transform_result| {
        let content = match transform_result {
            TransformResult::Chunk(content) | TransformResult::Error(content) => content,
        };

        Ok::<_, Error>(Bytes::from(format!("{content}\n")))
    });

    Ok(HttpResponse::Ok()
        .insert_header(header::ContentType::json())
        .insert_header((header::CACHE_CONTROL, "no-cache"))
        .streaming(stream))
}
