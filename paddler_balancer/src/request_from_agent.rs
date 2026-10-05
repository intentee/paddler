use std::fmt::Debug;
use std::sync::Arc;

use log::debug;
use log::error;
use log::warn;
use tokio::select;
use tokio_util::sync::CancellationToken;

use paddler_messaging::inference_client::message::Message as OutgoingMessage;
use paddler_messaging::inference_client::response::Response as OutgoingResponse;
use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
use paddler_messaging::streamable_result::StreamableResult;

use crate::agent_streaming_request::AgentStreamingRequest;
use crate::balancer_shutdown_error::balancer_shutdown_error;
use crate::buffered_request_agent_wait_result::BufferedRequestAgentWaitResult;
use crate::buffered_request_manager::BufferedRequestManager;
use crate::controls_session::ControlsSession;
use crate::dispatched_agent::DispatchedAgent;
use crate::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use crate::request_from_dispatched_agent::request_from_dispatched_agent;
use crate::respond_with_error::respond_with_error;

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

    select! {
        biased;

        () = shutdown.cancelled() => {
            respond_with_error(
                balancer_shutdown_error(),
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
                Ok(BufferedRequestAgentWaitResult::Timeout) => {
                    warn!("Buffered request {request_id:?} timed out waiting for an available slot");

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

pub async fn request_from_agent<TControlsSession, TParams>(
    buffered_request_manager: Arc<BufferedRequestManager>,
    connection_close: CancellationToken,
    inference_service_configuration: InferenceServiceConfiguration,
    params: TParams,
    request_id: String,
    mut session_controller: TControlsSession,
    shutdown: CancellationToken,
) where
    TControlsSession: ControlsSession<OutgoingMessage>,
    TParams: AgentStreamingRequest + Debug + Send,
    TParams::Response: Debug + Into<OutgoingResponse> + StreamableResult,
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

    request_from_dispatched_agent(
        dispatched_agent,
        connection_close,
        inference_service_configuration,
        params,
        request_id,
        session_controller,
        shutdown,
    )
    .await;
}
