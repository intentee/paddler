use std::fmt::Debug;
use std::sync::Arc;

use log::debug;
use log::error;
use log::warn;
use nanoid::nanoid;
use paddler_messaging::inference_client::message::Message as OutgoingMessage;
use paddler_messaging::inference_client::response::Response as OutgoingResponse;
use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
use paddler_messaging::jsonrpc::error_envelope::ErrorEnvelope;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_messaging::streamable_result::StreamableResult;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use crate::agent_controller::AgentController;
use crate::agent_response_forwarding_mode::AgentResponseForwardingMode;
use crate::agent_stop_outcome::AgentStopOutcome;
use crate::buffered_request_agent_wait_result::BufferedRequestAgentWaitResult;
use crate::buffered_request_manager::BufferedRequestManager;
use crate::controls_session::ControlsSession;
use crate::decide_forwarding_plan::decide_forwarding_plan;
use crate::dispatched_agent::DispatchedAgent;
use crate::forwarding_event::ForwardingEvent;
use crate::forwarding_plan::ForwardingPlan;
use crate::handles_agent_streaming_response::HandlesAgentStreamingResponse;
use crate::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use crate::manages_senders::ManagesSenders;
use crate::manages_senders_controller::ManagesSendersController;
use paddler_messaging::management_socket::agent::request::Request as AgentJsonRpcRequest;

pub async fn request_from_agent<TControlsSession, TParams>(
    buffered_request_manager: Arc<BufferedRequestManager>,
    connection_close: CancellationToken,
    inference_service_configuration: InferenceServiceConfiguration,
    params: TParams,
    request_id: String,
    mut session_controller: TControlsSession,
    shutdown: CancellationToken,
)
where
    TControlsSession: ControlsSession<OutgoingMessage>,
    TParams: Debug + Into<AgentJsonRpcRequest> + Send,
    AgentController: HandlesAgentStreamingResponse<TParams>,
    <<AgentController as HandlesAgentStreamingResponse<TParams>>::SenderCollection as ManagesSenders>::Value: Debug + Into<OutgoingResponse> + StreamableResult,
{
    let Some(dispatched_agent) = wait_for_agent_controller(
        buffered_request_manager.clone(),
        connection_close.clone(),
        request_id.clone(),
        &mut session_controller,
        shutdown.clone(),
    )
    .await
    else {
        return;
    };

    let receive_response_controller = match dispatched_agent
        .agent_controller
        .handle_streaming_response(nanoid!(), params)
        .await
    {
        Ok(receive_response_controller) => receive_response_controller,
        Err(err) => {
            error!("Failed to handle request {request_id:?}: {err}");

            respond_with_error(
                JsonRpcError {
                    code: 500,
                    description: "Failed to generate response".to_owned(),
                },
                request_id.clone(),
                &mut session_controller,
            )
            .await;

            return;
        }
    };

    forward_responses_stream(
        connection_close,
        dispatched_agent,
        inference_service_configuration,
        receive_response_controller,
        request_id,
        session_controller,
        shutdown,
    )
    .await;
}

pub async fn forward_responses_stream<TControlsSession, TManagesSenders>(
    connection_close: CancellationToken,
    dispatched_agent: DispatchedAgent,
    inference_service_configuration: InferenceServiceConfiguration,
    mut receive_response_controller: ManagesSendersController<TManagesSenders>,
    request_id: String,
    mut session_controller: TControlsSession,
    shutdown: CancellationToken,
) where
    TControlsSession: ControlsSession<OutgoingMessage>,
    TManagesSenders: ManagesSenders + Send + Sync,
    TManagesSenders::Value: Debug + Into<OutgoingResponse> + Send + StreamableResult,
{
    debug!("Found available agent controller for request: {request_id:?}");

    let agent_controller = dispatched_agent.agent_controller.clone();
    let agent_request_id = receive_response_controller.request_id.clone();
    let agent_connection_close = agent_controller.connection_close.clone();
    let inference_item_timeout = inference_service_configuration.inference_item_timeout;
    let mut forwarding_mode = AgentResponseForwardingMode::ForwardingToClient;

    loop {
        let is_forwarding_to_client = matches!(
            forwarding_mode,
            AgentResponseForwardingMode::ForwardingToClient
        );

        let forwarding_event = tokio::select! {
            biased;

            () = shutdown.cancelled() => ForwardingEvent::ShutdownRequested,
            () = agent_connection_close.cancelled() => ForwardingEvent::AgentConnectionClosed,
            () = connection_close.cancelled(), if is_forwarding_to_client => {
                ForwardingEvent::ClientConnectionClosed
            }
            () = sleep(inference_item_timeout) => ForwardingEvent::ItemTimedOut,
            Some(response) = receive_response_controller.response_rx.recv() => {
                ForwardingEvent::ResponseReceived(response)
            }
        };

        let agent_stop_required = match decide_forwarding_plan(
            forwarding_event,
            &forwarding_mode,
            inference_item_timeout,
        ) {
            ForwardingPlan::Finish => break,
            ForwardingPlan::IgnoreDrainedResponse => false,
            ForwardingPlan::ForwardResponse { is_done, response } => {
                let send_succeeded = send_response_to_client(
                    agent_controller.name.clone(),
                    response,
                    request_id.clone(),
                    &mut session_controller,
                )
                .await;

                if is_done {
                    break;
                }

                !send_succeeded
            }
            ForwardingPlan::ReplyWithErrorThenFinish(error) => {
                respond_with_error(error, request_id.clone(), &mut session_controller).await;

                break;
            }
            ForwardingPlan::ReplyWithErrorThenStopAgent(error) => {
                respond_with_error(error, request_id.clone(), &mut session_controller).await;

                true
            }
            ForwardingPlan::ReplyWithErrorThenStopAgentThenFinish(error) => {
                respond_with_error(error, request_id.clone(), &mut session_controller).await;
                stop_responding_to(&agent_controller, agent_request_id.clone()).await;

                break;
            }
            ForwardingPlan::StopAgent => true,
        };

        if agent_stop_required {
            match stop_responding_to(&agent_controller, agent_request_id.clone()).await {
                AgentStopOutcome::AgentUnreachable => break,
                AgentStopOutcome::StopRequested => {
                    forwarding_mode = AgentResponseForwardingMode::DrainingUntilAgentConfirms;
                }
            }
        }
    }
}

pub async fn respond_with_error<TControlsSession>(
    error: JsonRpcError,
    request_id: String,
    session_controller: &mut TControlsSession,
) where
    TControlsSession: ControlsSession<OutgoingMessage>,
{
    session_controller
        .send_response(OutgoingMessage::Error(ErrorEnvelope {
            request_id: request_id.clone(),
            error,
        }))
        .await
        .unwrap_or_else(|err| {
            error!("Failed to send response for request {request_id:?}: {err}");
        });
}

async fn send_response_to_client<TControlsSession, TResponse>(
    generated_by: Option<String>,
    response: TResponse,
    request_id: String,
    session_controller: &mut TControlsSession,
) -> bool
where
    TControlsSession: ControlsSession<OutgoingMessage>,
    TResponse: Into<OutgoingResponse> + Send,
{
    if let Err(err) = session_controller
        .send_response(OutgoingMessage::Response(ResponseEnvelope {
            generated_by,
            request_id: request_id.clone(),
            response: response.into(),
        }))
        .await
    {
        error!("Failed to send response for request {request_id:?}: {err}");

        return false;
    }

    true
}

async fn stop_responding_to(
    agent_controller: &AgentController,
    request_id: String,
) -> AgentStopOutcome {
    match agent_controller
        .stop_responding_to(request_id.clone())
        .await
    {
        Ok(()) => AgentStopOutcome::StopRequested,
        Err(err) => {
            error!("Failed to stop responding to request {request_id:?}: {err}");

            AgentStopOutcome::AgentUnreachable
        }
    }
}

async fn wait_for_agent_controller<TControlsSession>(
    buffered_request_manager: Arc<BufferedRequestManager>,
    connection_close: CancellationToken,
    request_id: String,
    session_controller: &mut TControlsSession,
    shutdown: CancellationToken,
) -> Option<DispatchedAgent>
where
    TControlsSession: ControlsSession<OutgoingMessage>,
{
    let buffered_request_manager = buffered_request_manager.clone();

    tokio::select! {
        biased;

        () = shutdown.cancelled() => {
            respond_with_error(
                JsonRpcError {
                    code: 503,
                    description: "balancer is shutting down".to_owned(),
                },
                request_id.clone(),
                session_controller,
            ).await;

            None
        },
        () = connection_close.cancelled() => {
            debug!("Connection close signal received, stopping GenerateTokens loop.");

            None
        },
        buffered_request_agent_wait_result = buffered_request_manager.wait_for_available_agent() => {
            match buffered_request_agent_wait_result {
                Ok(BufferedRequestAgentWaitResult::Found(dispatched_agent)) => Some(dispatched_agent),
                Ok(BufferedRequestAgentWaitResult::BufferOverflow) => {
                    warn!("Too many buffered requests, dropping request: {request_id:?}");

                    respond_with_error(
                        JsonRpcError {
                            code: 503,
                            description: "Buffered requests overflow".to_owned(),
                        },
                        request_id.clone(),
                        session_controller,
                    ).await;

                    None
                }
                Ok(BufferedRequestAgentWaitResult::Timeout(err)) => {
                    warn!("Buffered request {request_id:?} timed out: {err:?}");

                    respond_with_error(
                        JsonRpcError {
                            code: 504,
                            description: "Waiting for available slot timed out".to_owned(),
                        },
                        request_id.clone(),
                        session_controller,
                    ).await;

                    None
                }
                Err(err) => {
                    error!("Error while waiting for available agent controller for GenerateTokens request: {err}");

                    respond_with_error(
                        JsonRpcError {
                            code: 500,
                            description: "Internal server error".to_owned(),
                        },
                        request_id.clone(),
                        session_controller,
                    ).await;

                    None
                }
            }
        }
    }
}
