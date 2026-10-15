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

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;
    use std::sync::Arc;

    use serde_json::to_string;
    use tokio::sync::mpsc::unbounded_channel;
    use tokio_util::sync::CancellationToken;

    use paddler_messaging::inference_client::message::Message as OutgoingMessage;
    use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
    use paddler_messaging::jsonrpc::error_envelope::ErrorEnvelope;
    use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
    use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

    use super::request_from_dispatched_agent;
    use crate::agent_controller_fixture::AgentControllerFixture;
    use crate::agent_controller_pool_updates::AgentControllerPoolUpdates;
    use crate::agent_controller_slot_guard::AgentControllerSlotGuard;
    use crate::dispatched_agent::DispatchedAgent;
    use crate::unexpiring_inference_service_configuration::unexpiring_inference_service_configuration;
    use crate::websocket_outbound_frame::WebSocketOutboundFrame;
    use crate::websocket_session_controller::WebSocketSessionController;

    #[tokio::test]
    async fn a_request_for_an_agent_whose_socket_closed_is_answered_with_an_error() {
        let agent_controller = AgentControllerFixture::whose_socket_is_gone(
            "agent",
            SlotAggregatedStatusSnapshot::default(),
        );
        let (outbound_tx, mut outbound_rx) = unbounded_channel();

        agent_controller.slots_processing.increment();
        request_from_dispatched_agent(
            DispatchedAgent::new(
                agent_controller.clone(),
                AgentControllerSlotGuard::new(
                    agent_controller,
                    Arc::new(AgentControllerPoolUpdates::default()),
                ),
            ),
            CancellationToken::new(),
            unexpiring_inference_service_configuration(),
            ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::MIN,
                raw_prompt: "hello".to_owned(),
            },
            "client-request".to_owned(),
            WebSocketSessionController::<OutgoingMessage>::new(outbound_tx),
            CancellationToken::new(),
        )
        .await;

        assert_eq!(
            outbound_rx.try_recv(),
            Ok(WebSocketOutboundFrame::Text(
                to_string(&OutgoingMessage::Error(ErrorEnvelope {
                    error: JsonRpcError {
                        code: 500,
                        description: "Failed to generate response".to_owned(),
                    },
                    request_id: "client-request".to_owned(),
                }))
                .unwrap()
            ))
        );
    }
}
