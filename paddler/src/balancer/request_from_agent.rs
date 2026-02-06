use std::fmt::Debug;
use std::sync::Arc;

use anyhow::Result;
use log::debug;
use log::error;
use log::warn;
use tokio::sync::broadcast;
use tokio::time::sleep;

use crate::agent::jsonrpc::Request as AgentJsonRpcRequest;
use crate::balancer::agent_controller::AgentController;
use crate::balancer::buffered_request_agent_wait_result::BufferedRequestAgentWaitResult;
use crate::balancer::buffered_request_manager::BufferedRequestManager;
use crate::balancer::handles_agent_streaming_response::HandlesAgentStreamingResponse;
use crate::balancer::inference_client::Message as OutgoingMessage;
use crate::balancer::inference_client::Response as OutgoingResponse;
use crate::balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use crate::balancer::manages_senders::ManagesSenders;
use crate::balancer::manages_senders_controller::ManagesSendersController;
use crate::controls_session::ControlsSession;
use crate::jsonrpc::Error as JsonRpcError;
use crate::jsonrpc::ErrorEnvelope;
use crate::jsonrpc::ResponseEnvelope;
use crate::streamable_result::StreamableResult;

pub async fn request_from_agent<TControlsSession, TParams>(
    buffered_request_manager: Arc<BufferedRequestManager>,
    connection_close_tx: broadcast::Sender<()>,
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
        connection_close_tx.subscribe(),
        request_id.clone(),
        &mut session_controller,
    )
    .await?
    {
        Some(agent_controller) => {
            let receive_response_controller = match agent_controller
                .handle_streaming_response(request_id.clone(), params)
                .await
            {
                Ok(receive_response_controller) => receive_response_controller,
                Err(err) => {
                    error!("Failed to handle request {request_id:?}: {err}");

                    respond_with_error(
                        JsonRpcError {
                            code: 500,
                            description: "Failed to generate response".to_string(),
                        },
                        request_id.clone(),
                        &mut session_controller,
                    )
                    .await;

                    return Ok(());
                }
            };

            forward_responses_stream(
                agent_controller,
                connection_close_tx.subscribe(),
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
    mut connection_close_rx: broadcast::Receiver<()>,
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

    let mut agent_controller_connection_close_resubscribed =
        agent_controller.connection_close_rx.resubscribe();

    loop {
        tokio::select! {
            _ = agent_controller_connection_close_resubscribed.recv() => {
                error!("Agent controller connection closed");

                respond_with_error(
                    JsonRpcError {
                        code: 502,
                        description: "Agent controller connection closed".to_string(),
                    },
                    request_id,
                    &mut session_controller,
                ).await;

                break;
            }
            _ = connection_close_rx.recv() => {
                agent_controller.stop_responding_to(request_id.clone()).await.unwrap_or_else(|err| {
                    error!("Failed to stop request {request_id:?}: {err}");
                });

                break;
            }
            _ = sleep(inference_service_configuration.inference_item_timeout) => {
                warn!("Timed out waiting for response for request {request_id:?}");

                respond_with_error(
                    JsonRpcError {
                        code: 504,
                        description: "Downstream response timed out".to_string(),
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

                        send_response_to_client(
                            agent_controller.clone(),
                            response,
                            request_id.clone(),
                            &mut session_controller,
                        ).await;

                        if is_done {
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
) where
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
    }
}

async fn wait_for_agent_controller<TControlsSession>(
    buffered_request_manager: Arc<BufferedRequestManager>,
    mut connection_close_rx: broadcast::Receiver<()>,
    request_id: String,
    session_controller: &mut TControlsSession,
) -> Result<Option<Arc<AgentController>>>
where
    TControlsSession: ControlsSession<OutgoingMessage>,
{
    let buffered_request_manager = buffered_request_manager.clone();

    tokio::select! {
        _ = connection_close_rx.recv() => {
            debug!("Connection close signal received, stopping GenerateTokens loop.");

            Ok(None)
        },
        buffered_request_agent_wait_result = buffered_request_manager.wait_for_available_agent() => {
            match buffered_request_agent_wait_result {
                Ok(BufferedRequestAgentWaitResult::Found(agent_controller)) => Ok(Some(agent_controller)),
                Ok(BufferedRequestAgentWaitResult::BufferOverflow) => {
                    warn!("Too many buffered requests, dropping request: {request_id:?}");

                    respond_with_error(
                        JsonRpcError {
                            code: 503,
                            description: "Buffered requests overflow".to_string(),
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
                            description: "Waiting for available slot timed out".to_string(),
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
                            description: "Internal server error".to_string(),
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
