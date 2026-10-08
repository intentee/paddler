mod inference_socket_controller_context;
mod report_cluster_inference_mode;

use std::fmt::Debug;
use std::sync::Arc;

use actix_web::Error;
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use actix_web::rt;
use actix_web::web::Data;
use actix_web::web::Payload;
use actix_web::web::ServiceConfig;
use actix_web::web::get;
use actix_ws::Session;
use anyhow::Result;
use log::debug;
use log::error;
use serde_json::Error as SerdeJsonError;
use serde_json::from_str;
use tokio_util::sync::CancellationToken;

use paddler_messaging::api_path::ApiPath;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::inference_client::message::Message as OutgoingMessage;
use paddler_messaging::inference_client::response::Response as OutgoingResponse;
use paddler_messaging::inference_server::identified_request::IdentifiedRequest;
use paddler_messaging::inference_server::message::Message as InferenceServerMessage;
use paddler_messaging::inference_server::notification::Notification as InferenceServerNotification;
use paddler_messaging::inference_server::request::Request as InferenceServerRequest;
use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
use paddler_messaging::jsonrpc::request_envelope::RequestEnvelope;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;
use paddler_messaging::streamable_result::StreamableResult;
use paddler_messaging::validates::Validates as _;
use paddler_request_registry::request_registry_guard::RequestRegistryGuard;

use self::inference_socket_controller_context::InferenceSocketControllerContext;
use self::report_cluster_inference_mode::report_cluster_inference_mode;
use crate::agent_streaming_request::AgentStreamingRequest;
use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use crate::buffered_request_manager::BufferedRequestManager;
use crate::cluster_serves_another_inference_mode::ClusterServesAnotherInferenceMode;
use crate::continuation_decision::ContinuationDecision;
use crate::controls_session::ControlsSession as _;
use crate::controls_websocket_endpoint::ControlsWebSocketEndpoint;
use crate::inference_service::app_data::AppData;
use crate::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use crate::invalid_request_parameters_description::invalid_request_parameters_description;
use crate::request_from_agent::request_from_agent;
use crate::respond_with_error::respond_with_error;
use crate::websocket_session_controller::WebSocketSessionController;

async fn send_inference_mode_mismatch(
    cluster_serves_another_inference_mode: ClusterServesAnotherInferenceMode,
    request_id: String,
    websocket_session_controller: &mut WebSocketSessionController<OutgoingMessage>,
) {
    websocket_session_controller
        .send_response_safe(OutgoingMessage::Response(ResponseEnvelope {
            generated_by: None,
            request_id,
            response: OutgoingResponse::GeneratedToken(
                GeneratedTokenResult::InferenceModeMismatch(
                    cluster_serves_another_inference_mode.to_string(),
                ),
            ),
        }))
        .await;
}

async fn handle_inference_request<TParams>(
    connection_close: &CancellationToken,
    context: Arc<InferenceSocketControllerContext>,
    params: TParams,
    request_id: String,
    mut websocket_session_controller: WebSocketSessionController<OutgoingMessage>,
) where
    TParams: AgentStreamingRequest + Debug + Send + 'static,
    TParams::Response: Debug + Into<OutgoingResponse> + StreamableResult,
{
    if let Err(cluster_serves_another_inference_mode) = context
        .balancer_applicable_state_holder
        .require_inference_mode(TParams::INFERENCE_MODE)
    {
        send_inference_mode_mismatch(
            cluster_serves_another_inference_mode,
            request_id,
            &mut websocket_session_controller,
        )
        .await;

        return;
    }

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

        request_from_agent(
            context.buffered_request_manager.clone(),
            request_close,
            context.inference_service_configuration.clone(),
            params,
            request_id,
            websocket_session_controller,
            context.shutdown.clone(),
        )
        .await;
    });
}

async fn respond(
    app_data: Data<AppData>,
    payload: Payload,
    http_request: HttpRequest,
) -> Result<HttpResponse, Error> {
    let inference_socket_controller = InferenceSocketController {
        balancer_applicable_state_holder: app_data.balancer_applicable_state_holder.clone(),
        buffered_request_manager: app_data.buffered_request_manager.clone(),
        inference_service_configuration: app_data.inference_service_configuration.clone(),
        shutdown: app_data.shutdown.clone(),
    };

    inference_socket_controller.respond(payload, http_request, app_data.shutdown.clone())
}

type InferenceJsonRpcMessage = InferenceServerMessage<RawParametersSchema>;

type InferenceJsonRpcRequest = InferenceServerRequest<RawParametersSchema>;

struct InferenceSocketController {
    balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    buffered_request_manager: Arc<BufferedRequestManager>,
    inference_service_configuration: InferenceServiceConfiguration,
    shutdown: CancellationToken,
}

impl ControlsWebSocketEndpoint for InferenceSocketController {
    type Context = InferenceSocketControllerContext;
    type IncomingMessage = InferenceJsonRpcMessage;
    type OutgoingMessage = OutgoingMessage;

    fn create_context(&self) -> Self::Context {
        InferenceSocketControllerContext {
            balancer_applicable_state_holder: self.balancer_applicable_state_holder.clone(),
            buffered_request_manager: self.buffered_request_manager.clone(),
            inference_service_configuration: self.inference_service_configuration.clone(),
            request_cancellation_tokens: Arc::default(),
            shutdown: self.shutdown.clone(),
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
        connection_close: CancellationToken,
        context: Arc<Self::Context>,
        session: &mut Session,
    ) {
        report_cluster_inference_mode(
            context.balancer_applicable_state_holder.clone(),
            connection_close,
            session.clone(),
        )
        .await;
    }
}

pub fn ws_inference_socket(service_config: &mut ServiceConfig) {
    service_config.route(ApiPath::INFERENCE_SOCKET, get().to(respond));
}
