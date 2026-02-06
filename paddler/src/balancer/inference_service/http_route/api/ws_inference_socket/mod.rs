mod inference_socket_controller_context;
pub mod jsonrpc;

use std::sync::Arc;

use actix_web::Error;
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Payload;
use actix_web::web::ServiceConfig;
use anyhow::Result;
use async_trait::async_trait;
use log::error;
use tokio::sync::broadcast;

use self::inference_socket_controller_context::InferenceSocketControllerContext;
use self::jsonrpc::Message as InferenceJsonRpcMessage;
use self::jsonrpc::Request as InferenceJsonRpcRequest;
use crate::balancer::buffered_request_manager::BufferedRequestManager;
use crate::balancer::inference_client::Message as OutgoingMessage;
use crate::balancer::inference_service::app_data::AppData;
use crate::balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use crate::balancer::request_from_agent::request_from_agent;
use crate::controls_websocket_endpoint::ContinuationDecision;
use crate::controls_websocket_endpoint::ControlsWebSocketEndpoint;
use crate::jsonrpc::Error as JsonRpcError;
use crate::jsonrpc::ErrorEnvelope;
use crate::jsonrpc::RequestEnvelope;
use crate::validates::Validates as _;
use crate::websocket_session_controller::WebSocketSessionController;

pub fn register(cfg: &mut ServiceConfig) {
    cfg.service(respond);
}

struct InferenceSocketController {
    buffered_request_manager: Arc<BufferedRequestManager>,
    inference_service_configuration: InferenceServiceConfiguration,
}

#[async_trait]
impl ControlsWebSocketEndpoint for InferenceSocketController {
    type Context = InferenceSocketControllerContext;
    type IncomingMessage = InferenceJsonRpcMessage;
    type OutgoingMessage = OutgoingMessage;

    fn create_context(&self) -> Self::Context {
        InferenceSocketControllerContext {
            buffered_request_manager: self.buffered_request_manager.clone(),
            inference_service_configuration: self.inference_service_configuration.clone(),
        }
    }

    async fn handle_deserialized_message(
        connection_close_tx: broadcast::Sender<()>,
        context: Arc<Self::Context>,
        deserialized_message: Self::IncomingMessage,
        websocket_session_controller: WebSocketSessionController<Self::OutgoingMessage>,
    ) -> Result<ContinuationDecision> {
        match deserialized_message {
            InferenceJsonRpcMessage::Error(ErrorEnvelope {
                request_id,
                error: JsonRpcError { code, description },
            }) => {
                error!(
                    "Received error from client: code: {code}, description: {description:?}, request_id: {request_id:?}"
                );

                return Ok(ContinuationDecision::Continue);
            }
            InferenceJsonRpcMessage::Request(RequestEnvelope {
                id,
                request: InferenceJsonRpcRequest::ContinueFromConversationHistory(params),
            }) => {
                request_from_agent(
                    context.buffered_request_manager.clone(),
                    connection_close_tx,
                    context.inference_service_configuration.clone(),
                    params.validate()?,
                    id,
                    websocket_session_controller,
                )
                .await?;

                Ok(ContinuationDecision::Continue)
            }
            InferenceJsonRpcMessage::Request(RequestEnvelope {
                id,
                request: InferenceJsonRpcRequest::ContinueFromRawPrompt(params),
            }) => {
                request_from_agent(
                    context.buffered_request_manager.clone(),
                    connection_close_tx,
                    context.inference_service_configuration.clone(),
                    params,
                    id,
                    websocket_session_controller,
                )
                .await?;

                Ok(ContinuationDecision::Continue)
            }
        }
    }
}

#[get("/api/v1/inference_socket")]
async fn respond(
    app_data: Data<AppData>,
    payload: Payload,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let inference_socket_controller = InferenceSocketController {
        buffered_request_manager: app_data.buffered_request_manager.clone(),
        inference_service_configuration: app_data.inference_service_configuration.clone(),
    };

    inference_socket_controller.respond(payload, req)
}
