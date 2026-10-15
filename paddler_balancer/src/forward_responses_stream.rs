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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use llama_cpp_bindings_types::TokenUsage;
    use serde_json::to_string;
    use tokio::sync::mpsc::unbounded_channel;
    use tokio_util::sync::CancellationToken;

    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use paddler_messaging::generation_finish::GenerationFinish;
    use paddler_messaging::generation_summary::GenerationSummary;
    use paddler_messaging::inference_client::message::Message as OutgoingMessage;
    use paddler_messaging::inference_client::response::Response as OutgoingResponse;
    use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
    use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;
    use paddler_request_registry::request_delivery::RequestDelivery;

    use super::forward_responses_stream;
    use crate::agent_controller_fixture::AgentControllerFixture;
    use crate::agent_controller_pool_updates::AgentControllerPoolUpdates;
    use crate::agent_controller_slot_guard::AgentControllerSlotGuard;
    use crate::agent_response_receiver::AgentResponseReceiver;
    use crate::dispatched_agent::DispatchedAgent;
    use crate::unexpiring_inference_service_configuration::unexpiring_inference_service_configuration;
    use crate::websocket_outbound_frame::WebSocketOutboundFrame;
    use crate::websocket_session_controller::WebSocketSessionController;

    fn client_frame(generated_token_result: GeneratedTokenResult) -> WebSocketOutboundFrame {
        WebSocketOutboundFrame::Text(
            to_string(&OutgoingMessage::Response(ResponseEnvelope {
                generated_by: None,
                request_id: "client-request".to_owned(),
                response: OutgoingResponse::from(generated_token_result),
            }))
            .unwrap(),
        )
    }

    fn finished_generation() -> GeneratedTokenResult {
        GeneratedTokenResult::Done(GenerationSummary {
            finish: GenerationFinish::EndOfGeneration,
            usage: TokenUsage::new(),
        })
    }

    #[tokio::test]
    async fn forwarding_to_a_gone_client_for_a_gone_agent_releases_the_agent_slot() {
        let agent_controller = AgentControllerFixture::whose_socket_is_gone(
            "agent",
            SlotAggregatedStatusSnapshot::default(),
        );
        let receive_response_controller = AgentResponseReceiver::register(
            &agent_controller.agent_response_senders.generated_tokens,
            "agent-request".to_owned(),
        )
        .unwrap();
        let (outbound_tx, outbound_rx) = unbounded_channel();

        drop(outbound_rx);
        agent_controller.slots_processing.increment();

        assert_eq!(
            agent_controller
                .agent_response_senders
                .generated_tokens
                .send_to(
                    "agent-request",
                    GeneratedTokenResult::ContentToken("piece".to_owned())
                ),
            RequestDelivery::Delivered
        );

        forward_responses_stream(
            CancellationToken::new(),
            DispatchedAgent::new(
                agent_controller.clone(),
                AgentControllerSlotGuard::new(
                    agent_controller.clone(),
                    Arc::new(AgentControllerPoolUpdates::default()),
                ),
            ),
            unexpiring_inference_service_configuration(),
            receive_response_controller,
            "client-request".to_owned(),
            WebSocketSessionController::<OutgoingMessage>::new(outbound_tx),
            CancellationToken::new(),
        )
        .await;

        assert_eq!(agent_controller.slots_processing.get(), 0);
    }

    #[tokio::test]
    async fn a_client_that_reads_nothing_does_not_hold_the_agent_slot_after_the_generation() {
        let agent_controller =
            AgentControllerFixture::reporting("agent", SlotAggregatedStatusSnapshot::default())
                .agent_controller;
        let receive_response_controller = AgentResponseReceiver::register(
            &agent_controller.agent_response_senders.generated_tokens,
            "agent-request".to_owned(),
        )
        .unwrap();
        let (outbound_tx, mut unread_outbound_rx) = unbounded_channel();

        agent_controller.slots_processing.increment();

        for generated_token_result in [
            GeneratedTokenResult::ContentToken("piece".to_owned()),
            finished_generation(),
        ] {
            assert_eq!(
                agent_controller
                    .agent_response_senders
                    .generated_tokens
                    .send_to("agent-request", generated_token_result),
                RequestDelivery::Delivered
            );
        }

        forward_responses_stream(
            CancellationToken::new(),
            DispatchedAgent::new(
                agent_controller.clone(),
                AgentControllerSlotGuard::new(
                    agent_controller.clone(),
                    Arc::new(AgentControllerPoolUpdates::default()),
                ),
            ),
            unexpiring_inference_service_configuration(),
            receive_response_controller,
            "client-request".to_owned(),
            WebSocketSessionController::<OutgoingMessage>::new(outbound_tx),
            CancellationToken::new(),
        )
        .await;

        assert_eq!(agent_controller.slots_processing.get(), 0);
        assert_eq!(
            [
                unread_outbound_rx.try_recv().ok(),
                unread_outbound_rx.try_recv().ok(),
                unread_outbound_rx.try_recv().ok(),
            ],
            [
                Some(client_frame(GeneratedTokenResult::ContentToken(
                    "piece".to_owned()
                ))),
                Some(client_frame(finished_generation())),
                None,
            ]
        );
    }
}
