use std::fmt::Debug;

use log::error;
use nanoid::nanoid;
use tokio_util::sync::CancellationToken;

use paddler_messaging::inference_client::message::Message as OutgoingMessage;
use paddler_messaging::inference_client::response::Response as OutgoingResponse;
use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
use paddler_messaging::streamable_result::StreamableResult;

use crate::agent_streaming_request::AgentStreamingRequest;
use crate::controls_session::ControlsSession;
use crate::dispatched_agent::DispatchedAgent;
use crate::forward_responses_stream::forward_responses_stream;
use crate::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use crate::respond_with_error::respond_with_error;

pub async fn request_from_dispatched_agent<TControlsSession, TParams>(
    dispatched_agent: DispatchedAgent,
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
    let receive_response_controller = match dispatched_agent
        .agent_controller
        .stream_responses_to(nanoid!(), params)
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
