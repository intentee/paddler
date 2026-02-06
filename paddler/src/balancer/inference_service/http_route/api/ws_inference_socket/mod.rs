mod inference_socket_controller_context;

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
use paddler_types::inference_client::Message as OutgoingMessage;
use paddler_types::inference_server::Message as InferenceServerMessage;
use paddler_types::inference_server::Request as InferenceServerRequest;
use paddler_types::jsonrpc::Error as JsonRpcError;
use paddler_types::jsonrpc::ErrorEnvelope;
use paddler_types::jsonrpc::RequestEnvelope;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;
use paddler_types::validates::Validates as _;
use tokio::sync::broadcast;

use self::inference_socket_controller_context::InferenceSocketControllerContext;
use crate::balancer::buffered_request_manager::BufferedRequestManager;
use crate::balancer::inference_service::app_data::AppData;
use crate::balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use crate::balancer::request_from_agent::request_from_agent;
use crate::controls_websocket_endpoint::ContinuationDecision;
use crate::controls_websocket_endpoint::ControlsWebSocketEndpoint;
use crate::websocket_session_controller::WebSocketSessionController;

type InferenceJsonRpcMessage = InferenceServerMessage<RawParametersSchema>;
type InferenceJsonRpcRequest = InferenceServerRequest<RawParametersSchema>;

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
                id: request_id,
                request:
                    InferenceJsonRpcRequest::ContinueFromConversationHistory(
                        conversation_history_params,
                    ),
            }) => {
                request_from_agent(
                    context.buffered_request_manager.clone(),
                    connection_close_tx,
                    context.inference_service_configuration.clone(),
                    conversation_history_params.validate()?,
                    request_id,
                    websocket_session_controller,
                )
                .await?;

                Ok(ContinuationDecision::Continue)
            }
            InferenceJsonRpcMessage::Request(RequestEnvelope {
                id: request_id,
                request: InferenceJsonRpcRequest::ContinueFromRawPrompt(raw_prompt_params),
            }) => {
                request_from_agent(
                    context.buffered_request_manager.clone(),
                    connection_close_tx,
                    context.inference_service_configuration.clone(),
                    raw_prompt_params,
                    request_id,
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
    http_request: HttpRequest,
) -> Result<HttpResponse, Error> {
    let inference_socket_controller = InferenceSocketController {
        buffered_request_manager: app_data.buffered_request_manager.clone(),
        inference_service_configuration: app_data.inference_service_configuration.clone(),
    };

    inference_socket_controller.respond(payload, http_request)
}

pub fn register(service_config: &mut ServiceConfig) {
    service_config.service(respond);
}
