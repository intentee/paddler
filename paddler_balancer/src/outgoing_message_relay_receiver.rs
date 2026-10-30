use paddler_messaging::inference_client::message::Message as OutgoingMessage;
use paddler_messaging::inference_client::response::Response as OutgoingResponse;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_messaging::streamable_result::StreamableResult;

use crate::agent_relay_conclusion::AgentRelayConclusion;
use crate::controls_session::ControlsSession;
use crate::receives_agent_relay::ReceivesAgentRelay;
use crate::respond_with_error::respond_with_error;

pub struct OutgoingMessageRelayReceiver<TControlsSession> {
    pub request_id: String,
    pub session_controller: TControlsSession,
}

impl<TControlsSession> OutgoingMessageRelayReceiver<TControlsSession>
where
    TControlsSession: ControlsSession<OutgoingMessage>,
{
    async fn send_response(&mut self, generated_by: Option<String>, response: OutgoingResponse) {
        self.session_controller
            .send_response_safe(OutgoingMessage::Response(ResponseEnvelope {
                generated_by,
                request_id: self.request_id.clone(),
                response,
            }))
            .await;
    }
}

impl<TControlsSession, TResponse> ReceivesAgentRelay<TResponse>
    for OutgoingMessageRelayReceiver<TControlsSession>
where
    TControlsSession: ControlsSession<OutgoingMessage>,
    TResponse: StreamableResult<Final: Send, Partial: Send> + Into<OutgoingResponse>,
{
    async fn receive_conclusion(mut self, conclusion: AgentRelayConclusion<TResponse::Final>) {
        match conclusion {
            AgentRelayConclusion::AgentConcluded {
                final_result,
                generated_by,
            } => {
                self.send_response(generated_by, Into::<TResponse>::into(final_result).into())
                    .await;
            }
            AgentRelayConclusion::Failed(relay_failure) => {
                respond_with_error(
                    relay_failure.into_jsonrpc_error(),
                    self.request_id,
                    &mut self.session_controller,
                )
                .await;
            }
        }
    }

    async fn receive_partial(
        &mut self,
        generated_by: Option<String>,
        partial_result: TResponse::Partial,
    ) {
        self.send_response(generated_by, Into::<TResponse>::into(partial_result).into())
            .await;
    }
}
