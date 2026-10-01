use std::fmt::Debug;
use std::pin::pin;

use log::debug;
use log::error;
use tokio::select;
use tokio::time::Instant;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use paddler_messaging::inference_client::message::Message as OutgoingMessage;
use paddler_messaging::inference_client::response::Response as OutgoingResponse;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_messaging::streamable_result::StreamableResult;

use crate::agent_response_forwarding_mode::AgentResponseForwardingMode;
use crate::agent_response_receiver::AgentResponseReceiver;
use crate::agent_stop_follow_up::AgentStopFollowUp;
use crate::controls_session::ControlsSession;
use crate::dispatched_agent::DispatchedAgent;
use crate::forwarding_event::ForwardingEvent;
use crate::forwarding_plan::ForwardingPlan;
use crate::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use crate::respond_with_error::respond_with_error;

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

pub async fn forward_responses_stream<TControlsSession, TResponse>(
    connection_close: CancellationToken,
    dispatched_agent: DispatchedAgent,
    inference_service_configuration: InferenceServiceConfiguration,
    mut receive_response_controller: AgentResponseReceiver<TResponse>,
    request_id: String,
    mut session_controller: TControlsSession,
    shutdown: CancellationToken,
) where
    TControlsSession: ControlsSession<OutgoingMessage>,
    TResponse: Debug + Into<OutgoingResponse> + Send + StreamableResult,
{
    debug!("Found available agent controller for request: {request_id:?}");

    let agent_controller = dispatched_agent.agent_controller.clone();
    let agent_request_id = receive_response_controller
        .request_registration
        .request_id
        .clone();
    let agent_connection_close = agent_controller.connection_close.clone();
    let inference_item_timeout = inference_service_configuration.inference_item_timeout;
    let mut forwarding_mode = AgentResponseForwardingMode::ForwardingToClient;
    let mut item_timeout = pin!(sleep(inference_item_timeout));

    loop {
        item_timeout
            .as_mut()
            .reset(Instant::now() + inference_item_timeout);

        let is_forwarding_to_client = matches!(
            forwarding_mode,
            AgentResponseForwardingMode::ForwardingToClient
        );

        let forwarding_event = select! {
            biased;

            () = shutdown.cancelled() => ForwardingEvent::ShutdownRequested,
            () = agent_connection_close.cancelled() => ForwardingEvent::AgentConnectionClosed,
            () = connection_close.cancelled(), if is_forwarding_to_client => {
                ForwardingEvent::ClientConnectionClosed
            }
            () = &mut item_timeout => ForwardingEvent::ItemTimedOut,
            Some(response) = receive_response_controller.response_rx.recv() => {
                ForwardingEvent::ResponseReceived(response)
            }
        };

        let agent_stop_follow_up =
            match forwarding_mode.plan_for(forwarding_event, inference_item_timeout) {
                ForwardingPlan::Finish => break,
                ForwardingPlan::IgnoreDrainedResponse => continue,
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

                    if send_succeeded {
                        continue;
                    }

                    AgentStopFollowUp::DrainUntilAgentConfirms
                }
                ForwardingPlan::ReplyWithErrorThenFinish(error) => {
                    respond_with_error(error, request_id.clone(), &mut session_controller).await;

                    break;
                }
                ForwardingPlan::ReplyWithErrorThenStopAgent(error) => {
                    respond_with_error(error, request_id.clone(), &mut session_controller).await;

                    AgentStopFollowUp::DrainUntilAgentConfirms
                }
                ForwardingPlan::ReplyWithErrorThenStopAgentThenFinish(error) => {
                    respond_with_error(error, request_id.clone(), &mut session_controller).await;

                    AgentStopFollowUp::Finish
                }
                ForwardingPlan::StopAgent => AgentStopFollowUp::DrainUntilAgentConfirms,
            };

        if let Err(stop_error) = agent_controller.stop_responding_to(agent_request_id.clone()) {
            error!("Failed to stop responding to request {request_id:?}: {stop_error}");

            break;
        }

        match agent_stop_follow_up {
            AgentStopFollowUp::DrainUntilAgentConfirms => {
                forwarding_mode = AgentResponseForwardingMode::DrainingUntilAgentConfirms;
            }
            AgentStopFollowUp::Finish => break,
        }
    }
}
