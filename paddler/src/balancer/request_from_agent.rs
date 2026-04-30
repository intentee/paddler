use std::fmt::Debug;
use std::sync::Arc;

use anyhow::Result;
use log::debug;
use log::error;
use log::warn;
use paddler_types::inference_client::Message as OutgoingMessage;
use paddler_types::inference_client::Response as OutgoingResponse;
use paddler_types::jsonrpc::Error as JsonRpcError;
use paddler_types::jsonrpc::ErrorEnvelope;
use paddler_types::jsonrpc::ResponseEnvelope;
use paddler_types::streamable_result::StreamableResult;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use crate::agent::jsonrpc::Request as AgentJsonRpcRequest;
use crate::balancer::agent_controller::AgentController;
use crate::balancer::buffered_request_agent_wait_result::BufferedRequestAgentWaitResult;
use crate::balancer::buffered_request_manager::BufferedRequestManager;
use crate::balancer::dispatched_agent::DispatchedAgent;
use crate::balancer::handles_agent_streaming_response::HandlesAgentStreamingResponse;
use crate::balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use crate::balancer::manages_senders::ManagesSenders;
use crate::balancer::manages_senders_controller::ManagesSendersController;
use crate::controls_session::ControlsSession;

pub async fn request_from_agent<TControlsSession, TParams>(
    buffered_request_manager: Arc<BufferedRequestManager>,
    connection_close: CancellationToken,
    inference_service_configuration: InferenceServiceConfiguration,
    params: TParams,
    request_id: String,
    mut session_controller: TControlsSession,
) -> Result<()>
where
    TControlsSession: ControlsSession<OutgoingMessage>,
    TParams: Debug + Into<AgentJsonRpcRequest> + Send,
    AgentController: HandlesAgentStreamingResponse<TParams>,
    <<AgentController as HandlesAgentStreamingResponse<TParams>>::SenderCollection as ManagesSenders>::Value: Debug + Into<OutgoingResponse> + StreamableResult,
{
    match wait_for_agent_controller(
        buffered_request_manager.clone(),
        connection_close.clone(),
        request_id.clone(),
        &mut session_controller,
    )
    .await?
    {
        Some(dispatched_agent) => {
            let receive_response_controller = match dispatched_agent
                .agent_controller
                .handle_streaming_response(request_id.clone(), params)
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

                    return Ok(());
                }
            };

            forward_responses_stream(
                dispatched_agent.agent_controller.clone(),
                connection_close,
                inference_service_configuration,
                receive_response_controller,
                request_id,
                session_controller,
            )
            .await?;

            Ok(())
        }
        None => Ok(()),
    }
}

async fn forward_responses_stream<TControlsSession, TManagesSenders>(
    agent_controller: Arc<AgentController>,
    connection_close: CancellationToken,
    inference_service_configuration: InferenceServiceConfiguration,
    mut receive_response_controller: ManagesSendersController<TManagesSenders>,
    request_id: String,
    mut session_controller: TControlsSession,
) -> Result<()>
where
    TControlsSession: ControlsSession<OutgoingMessage>,
    TManagesSenders: ManagesSenders + Send + Sync,
    TManagesSenders::Value: Debug + Into<OutgoingResponse> + Send + StreamableResult,
{
    debug!("Found available agent controller for request: {request_id:?}");

    let agent_connection_close = agent_controller.connection_close.clone();

    loop {
        tokio::select! {
            () = agent_connection_close.cancelled() => {
                error!("Agent controller connection closed");

                respond_with_error(
                    JsonRpcError {
                        code: 502,
                        description: "Agent controller connection closed".to_owned(),
                    },
                    request_id,
                    &mut session_controller,
                ).await;

                break;
            }
            () = connection_close.cancelled() => {
                agent_controller.stop_responding_to(request_id.clone()).await.unwrap_or_else(|err| {
                    error!("Failed to stop request {request_id:?}: {err}");
                });

                break;
            }
            () = sleep(inference_service_configuration.inference_item_timeout) => {
                let timeout_ms = inference_service_configuration.inference_item_timeout.as_millis();

                warn!(
                    "Timed out after {timeout_ms}ms waiting for next token for request {request_id:?}. \
                    Consider increasing --inference-item-timeout if the model needs more time to process the prompt."
                );

                respond_with_error(
                    JsonRpcError {
                        code: 504,
                        description: format!(
                            "Inference timed out after {timeout_ms}ms waiting for next token. \
                            Increase --inference-item-timeout if the prompt requires longer processing."
                        ),
                    },
                    request_id.clone(),
                    &mut session_controller,
                ).await;

                agent_controller.stop_responding_to(request_id.clone()).await.unwrap_or_else(|err| {
                    error!("Failed to stop responding to request {request_id:?}: {err}");
                });

                break;
            }
            response = receive_response_controller.response_rx.recv() => {
                match response {
                    Some(response) => {
                        let is_done = response.is_done();

                        let send_succeeded = send_response_to_client(
                            agent_controller.clone(),
                            response,
                            request_id.clone(),
                            &mut session_controller,
                        ).await;

                        if is_done || !send_succeeded {
                            break;
                        }
                    }
                    None => break,
                }
            }
        }
    }

    Ok(())
}

async fn respond_with_error<TControlsSession>(
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
    agent_controller: Arc<AgentController>,
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
            request_id: request_id.clone(),
            response: response.into(),
        }))
        .await
    {
        error!("Failed to send response for request {request_id:?}: {err}");

        agent_controller
            .stop_responding_to(request_id.clone())
            .await
            .unwrap_or_else(|err| {
                error!("Failed to stop responding to request {request_id:?}: {err}");
            });

        return false;
    }

    true
}

async fn wait_for_agent_controller<TControlsSession>(
    buffered_request_manager: Arc<BufferedRequestManager>,
    connection_close: CancellationToken,
    request_id: String,
    session_controller: &mut TControlsSession,
) -> Result<Option<DispatchedAgent>>
where
    TControlsSession: ControlsSession<OutgoingMessage>,
{
    let buffered_request_manager = buffered_request_manager.clone();

    tokio::select! {
        () = connection_close.cancelled() => {
            debug!("Connection close signal received, stopping GenerateTokens loop.");

            Ok(None)
        },
        buffered_request_agent_wait_result = buffered_request_manager.wait_for_available_agent() => {
            match buffered_request_agent_wait_result {
                Ok(BufferedRequestAgentWaitResult::Found(dispatched_agent)) => Ok(Some(dispatched_agent)),
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

                    Ok(None)
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

                    Ok(None)
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

                    Ok(None)
                }
            }
        }
    }
}
