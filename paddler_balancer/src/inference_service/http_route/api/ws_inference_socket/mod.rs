mod inference_socket_controller_context;
mod spawn_token_generation_mode_watcher;

use std::fmt::Debug;
use std::sync::Arc;

use actix_web::rt;
use actix_web::Error;
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Payload;
use actix_web::web::ServiceConfig;
use actix_ws::Session;
use anyhow::Result;
use async_trait::async_trait;
use log::error;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::inference_client::message::Message as OutgoingMessage;
use paddler_messaging::inference_client::response::Response as OutgoingResponse;
use paddler_messaging::inference_server::message::Message as InferenceServerMessage;
use paddler_messaging::inference_server::notification::Notification as InferenceServerNotification;
use paddler_messaging::inference_server::request::Request as InferenceServerRequest;
use paddler_messaging::management_socket::agent::request::Request as AgentJsonRpcRequest;
use paddler_messaging::streamable_result::StreamableResult;
use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
use paddler_messaging::jsonrpc::request_envelope::RequestEnvelope;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;
use paddler_messaging::validates::Validates as _;
use tokio_util::sync::CancellationToken;

use self::inference_socket_controller_context::InferenceSocketControllerContext;
use self::spawn_token_generation_mode_watcher::spawn_token_generation_mode_watcher;
use crate::agent_controller::AgentController;
use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use crate::buffered_request_manager::BufferedRequestManager;
use crate::cluster_token_generation_mode::ClusterTokenGenerationMode;
use crate::cluster_token_generation_mode::TOKEN_GENERATION_DISABLED_MESSAGE;
use crate::continuation_decision::ContinuationDecision;
use crate::controls_session::ControlsSession as _;
use crate::controls_websocket_endpoint::ControlsWebSocketEndpoint;
use crate::handles_agent_streaming_response::HandlesAgentStreamingResponse;
use crate::inference_service::app_data::AppData;
use crate::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use crate::invalid_request_parameters_description::invalid_request_parameters_description;
use crate::manages_senders::ManagesSenders;
use crate::request_cancellation_registration::RequestCancellationRegistration;
use crate::request_cancellation_token_guard::RequestCancellationTokenGuard;
use crate::request_cancellation_tokens::RequestCancellationTokens;
use crate::request_from_agent::request_from_agent;
use crate::request_from_agent::respond_with_error;
use crate::websocket_session_controller::WebSocketSessionController;

type InferenceJsonRpcMessage = InferenceServerMessage<RawParametersSchema>;
type InferenceJsonRpcRequest = InferenceServerRequest<RawParametersSchema>;

async fn send_token_generation_disabled(
    request_id: String,
    websocket_session_controller: &mut WebSocketSessionController<OutgoingMessage>,
) {
    if let Err(err) = websocket_session_controller
        .send_response(OutgoingMessage::Response(ResponseEnvelope {
            generated_by: None,
            request_id: request_id.clone(),
            response: OutgoingResponse::GeneratedToken(
                GeneratedTokenResult::TokenGenerationDisabled(
                    TOKEN_GENERATION_DISABLED_MESSAGE.to_owned(),
                ),
            ),
        }))
        .await
    {
        error!(
            "Failed to send token-generation-disabled response for request {request_id:?}: {err}"
        );
    }
}

async fn handle_inference_request<TParams>(
    connection_close: &CancellationToken,
    context: Arc<InferenceSocketControllerContext>,
    params: TParams,
    request_id: String,
    mut websocket_session_controller: WebSocketSessionController<OutgoingMessage>,
) where
    TParams: Debug + Into<AgentJsonRpcRequest> + Send + 'static,
    AgentController: HandlesAgentStreamingResponse<TParams>,
    <<AgentController as HandlesAgentStreamingResponse<TParams>>::SenderCollection as ManagesSenders>::Value: Debug + Into<OutgoingResponse> + StreamableResult,
{
    match ClusterTokenGenerationMode::from_applicable_state_holder(
        &context.balancer_applicable_state_holder,
    ) {
        ClusterTokenGenerationMode::DisabledForEmbeddings => {
            send_token_generation_disabled(request_id, &mut websocket_session_controller).await;
        }
        ClusterTokenGenerationMode::Enabled => {
            match RequestCancellationTokenGuard::register(
                connection_close,
                context.request_cancellation_tokens.clone(),
                request_id.clone(),
            ) {
                RequestCancellationRegistration::Registered(request_cancellation_token_guard) => {
                    rt::spawn(async move {
                        request_from_agent(
                            context.buffered_request_manager.clone(),
                            request_cancellation_token_guard.cancellation_token.clone(),
                            context.inference_service_configuration.clone(),
                            params,
                            request_id,
                            websocket_session_controller,
                            context.shutdown.clone(),
                        )
                        .await;
                    });
                }
                RequestCancellationRegistration::DuplicateRequestId => {
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
                }
            }
        }
    }
}

struct InferenceSocketController {
    balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    buffered_request_manager: Arc<BufferedRequestManager>,
    inference_service_configuration: InferenceServiceConfiguration,
    shutdown: CancellationToken,
}

#[async_trait]
impl ControlsWebSocketEndpoint for InferenceSocketController {
    type Context = InferenceSocketControllerContext;
    type IncomingMessage = InferenceJsonRpcMessage;
    type OutgoingMessage = OutgoingMessage;

    fn create_context(&self) -> Self::Context {
        InferenceSocketControllerContext {
            balancer_applicable_state_holder: self.balancer_applicable_state_holder.clone(),
            buffered_request_manager: self.buffered_request_manager.clone(),
            inference_service_configuration: self.inference_service_configuration.clone(),
            request_cancellation_tokens: Arc::new(RequestCancellationTokens::default()),
            shutdown: self.shutdown.clone(),
        }
    }

    async fn handle_deserialized_message(
        connection_close: CancellationToken,
        context: Arc<Self::Context>,
        deserialized_message: Self::IncomingMessage,
        websocket_session_controller: WebSocketSessionController<Self::OutgoingMessage>,
    ) -> Result<ContinuationDecision> {
        match deserialized_message {
            InferenceJsonRpcMessage::Notification(
                InferenceServerNotification::StopRespondingTo(request_id),
            ) => {
                context.request_cancellation_tokens.cancel(&request_id);
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

        Ok(ContinuationDecision::Continue)
    }

    async fn on_connection_start(
        connection_close: CancellationToken,
        context: Arc<Self::Context>,
        session: &mut Session,
    ) -> ContinuationDecision {
        spawn_token_generation_mode_watcher(
            context.balancer_applicable_state_holder.clone(),
            connection_close,
            session.clone(),
        );

        ContinuationDecision::Continue
    }
}

#[get("/api/v1/inference_socket")]
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

pub fn register(service_config: &mut ServiceConfig) {
    service_config.service(respond);
}
