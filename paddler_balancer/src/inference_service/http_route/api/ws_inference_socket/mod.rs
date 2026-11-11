mod inference_socket_controller_context;

use std::future::Ready;
use std::future::ready;
use std::sync::Arc;

use actix_web::Error;
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use actix_web::rt;
use actix_web::web::Data;
use actix_web::web::Payload;
use actix_web::web::ServiceConfig;
use actix_web::web::get;
use anyhow::Result;
use log::debug;
use log::error;
use serde_json::Error as SerdeJsonError;
use serde_json::from_str;
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;

use paddler_messaging::constants::API_PATH_INFERENCE_SOCKET;
use paddler_messaging::inference_client::message::Message as OutgoingMessage;
use paddler_messaging::inference_server::identified_request::IdentifiedRequest;
use paddler_messaging::inference_server::message::Message as InferenceServerMessage;
use paddler_messaging::inference_server::notification::Notification as InferenceServerNotification;
use paddler_messaging::inference_server::request::Request as InferenceServerRequest;
use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
use paddler_messaging::jsonrpc::request_envelope::RequestEnvelope;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;
use paddler_messaging::validates::Validates as _;
use paddler_request_registry::request_registry_guard::RequestRegistryGuard;

use self::inference_socket_controller_context::InferenceSocketControllerContext;
use crate::agent_relay::AgentRelay;
use crate::agent_streaming_request::AgentStreamingRequest;
use crate::awaiting_agent_relay::AwaitingAgentRelay;
use crate::continuation_decision::ContinuationDecision;
use crate::controls_websocket_endpoint::ControlsWebSocketEndpoint;
use crate::dispatchable_request::DispatchableRequest;
use crate::inference_service::app_data::AppData;
use crate::invalid_request_parameters_description::invalid_request_parameters_description;
use crate::outgoing_message_relay_receiver::OutgoingMessageRelayReceiver;
use crate::respond_with_error::respond_with_error;
use crate::websocket_outbound_frame::WebSocketOutboundFrame;
use crate::websocket_session_controller::WebSocketSessionController;

async fn handle_inference_request<TParams>(
    connection_close: &CancellationToken,
    context: Arc<InferenceSocketControllerContext>,
    params: TParams,
    request_id: String,
    mut websocket_session_controller: WebSocketSessionController<OutgoingMessage>,
) where
    TParams: AgentStreamingRequest + Send + 'static,
{
    let request_close = connection_close.child_token();

    let Some(request_registration) = RequestRegistryGuard::register(
        &context.request_cancellation_tokens,
        request_id.clone(),
        request_close.clone(),
    ) else {
        error!("Rejecting duplicate inference request {request_id:?}");

        respond_with_error(
            JsonRpcError {
                code: 400,
                description: format!(
                    "Request id {request_id:?} is already in flight on this connection"
                ),
            },
            request_id,
            &mut websocket_session_controller,
        )
        .await;

        return;
    };

    rt::spawn(async move {
        let _request_registration = request_registration;

        AgentRelay::awaiting(AwaitingAgentRelay {
            buffered_request_manager: context.buffered_request_manager.clone(),
            dispatchable_request: DispatchableRequest::from(params),
            inference_item_timeout: context
                .inference_service_configuration
                .inference_item_timeout,
            shutdown: context.shutdown.clone(),
        })
        .forward(
            request_close,
            OutgoingMessageRelayReceiver {
                request_id,
                session_controller: websocket_session_controller,
            },
        )
        .await;
    });
}

fn respond(
    app_data: Data<AppData>,
    payload: Payload,
    http_request: HttpRequest,
) -> Ready<Result<HttpResponse, Error>> {
    let shutdown = app_data.shutdown.clone();

    ready(InferenceSocketController { app_data }.respond(payload, http_request, shutdown))
}

type InferenceJsonRpcMessage = InferenceServerMessage<RawParametersSchema>;

type InferenceJsonRpcRequest = InferenceServerRequest<RawParametersSchema>;

struct InferenceSocketController {
    app_data: Data<AppData>,
}

impl ControlsWebSocketEndpoint for InferenceSocketController {
    type Context = InferenceSocketControllerContext;
    type IncomingMessage = InferenceJsonRpcMessage;
    type OutgoingMessage = OutgoingMessage;

    fn create_context(&self) -> Self::Context {
        InferenceSocketControllerContext {
            buffered_request_manager: self.app_data.buffered_request_manager.clone(),
            inference_service_configuration: self.app_data.inference_service_configuration.clone(),
            request_cancellation_tokens: Arc::default(),
            shutdown: self.app_data.shutdown.clone(),
        }
    }

    async fn handle_deserialized_message(
        connection_close: CancellationToken,
        context: Arc<Self::Context>,
        deserialized_message: Self::IncomingMessage,
        websocket_session_controller: WebSocketSessionController<Self::OutgoingMessage>,
    ) -> ContinuationDecision {
        match deserialized_message {
            InferenceJsonRpcMessage::Notification(
                InferenceServerNotification::StopRespondingTo(request_id),
            ) => {
                if context
                    .request_cancellation_tokens
                    .with_registered(&request_id, CancellationToken::cancel)
                    .is_none()
                {
                    debug!(
                        "Received a stop request for an unknown or already finished request: {request_id:?}"
                    );
                }
            }
            InferenceJsonRpcMessage::Request(RequestEnvelope {
                id: request_id,
                request:
                    InferenceJsonRpcRequest::ContinueFromConversationHistory(
                        conversation_history_params,
                    ),
            }) => match conversation_history_params.validate() {
                Ok(validated_params) => {
                    handle_inference_request(
                        &connection_close,
                        context,
                        validated_params,
                        request_id,
                        websocket_session_controller,
                    )
                    .await;
                }
                Err(validation_error) => {
                    let mut websocket_session_controller = websocket_session_controller;

                    respond_with_error(
                        JsonRpcError {
                            code: 400,
                            description: invalid_request_parameters_description(&validation_error),
                        },
                        request_id,
                        &mut websocket_session_controller,
                    )
                    .await;
                }
            },
            InferenceJsonRpcMessage::Request(RequestEnvelope {
                id: request_id,
                request: InferenceJsonRpcRequest::ContinueFromRawPrompt(raw_prompt_params),
            }) => {
                handle_inference_request(
                    &connection_close,
                    context,
                    raw_prompt_params,
                    request_id,
                    websocket_session_controller,
                )
                .await;
            }
        }

        ContinuationDecision::Continue
    }

    async fn handle_undeserializable_message(
        text: &str,
        deserialization_error: SerdeJsonError,
        mut websocket_session_controller: WebSocketSessionController<Self::OutgoingMessage>,
    ) -> ContinuationDecision {
        match from_str::<IdentifiedRequest>(text) {
            Ok(IdentifiedRequest::Request { id }) => {
                respond_with_error(
                    JsonRpcError {
                        code: 400,
                        description: format!(
                            "Request could not be deserialized: {deserialization_error}"
                        ),
                    },
                    id,
                    &mut websocket_session_controller,
                )
                .await;
            }
            Err(identification_error) => {
                error!(
                    "Unable to answer an undeserializable message without a request id: {identification_error}"
                );
            }
        }

        ContinuationDecision::Continue
    }

    async fn on_connection_start(
        _connection_close: CancellationToken,
        _context: Arc<Self::Context>,
        _outbound_tx: &UnboundedSender<WebSocketOutboundFrame>,
    ) {
    }
}

pub fn ws_inference_socket(service_config: &mut ServiceConfig) {
    service_config.route(API_PATH_INFERENCE_SOCKET, get().to(respond));
}
