use std::fmt::Debug;
use std::sync::Arc;

use actix_web::Error;
use actix_web::rt;
use log::error;
use nanoid::nanoid;
use paddler_types::inference_client::Message as OutgoingMessage;
use paddler_types::inference_client::Response as OutgoingResponse;
use paddler_types::jsonrpc::Error as JsonRpcError;
use paddler_types::jsonrpc::ErrorEnvelope;
use paddler_types::streamable_result::StreamableResult;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::agent::jsonrpc::Request as AgentJsonRpcRequest;
use crate::balancer::agent_controller::AgentController;
use crate::balancer::buffered_request_manager::BufferedRequestManager;
use crate::balancer::chunk_forwarding_session_controller::ChunkForwardingSessionController;
use crate::balancer::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;
use crate::balancer::handles_agent_streaming_response::HandlesAgentStreamingResponse;
use crate::balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use crate::balancer::manages_senders::ManagesSenders;
use crate::balancer::request_from_agent::request_from_agent;
use crate::controls_session::ControlsSession as _;

pub fn unbounded_stream_from_agent<TParams, TTransformsOutgoingMessage>(
    buffered_request_manager: Arc<BufferedRequestManager>,
    inference_service_configuration: InferenceServiceConfiguration,
    params: TParams,
    transformer: TTransformsOutgoingMessage,
) -> Result<UnboundedReceiverStream<String>, Error>
where
    TParams: Debug + Into<AgentJsonRpcRequest> + Send + 'static,
    AgentController: HandlesAgentStreamingResponse<TParams>,
    <<AgentController as HandlesAgentStreamingResponse<TParams>>::SenderCollection as ManagesSenders>::Value: Debug + Into<OutgoingResponse> + StreamableResult,
    TTransformsOutgoingMessage: Clone + TransformsOutgoingMessage + Send + Sync + 'static,
{
    let request_id: String = nanoid!();
    let (connection_close_tx, _connection_close_rx) = broadcast::channel(1);
    let (chunk_tx, chunk_rx) = mpsc::unbounded_channel::<String>();

    rt::spawn(async move {
        let mut session_controller = ChunkForwardingSessionController::new(chunk_tx, transformer);

        if let Err(err) = request_from_agent(
            buffered_request_manager.clone(),
            connection_close_tx,
            inference_service_configuration.clone(),
            params,
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

    Ok(UnboundedReceiverStream::new(chunk_rx))
}
