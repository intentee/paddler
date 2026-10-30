use tokio::select;
use tokio_util::sync::CancellationToken;

use crate::agent_relay::AgentRelay;
use crate::agent_relay_step::AgentRelayStep;
use crate::agent_streaming_request::AgentStreamingRequest;
use crate::receives_agent_relay::ReceivesAgentRelay;

async fn relay_to_receiver<TParams, TReceiver>(
    mut agent_relay: AgentRelay<TParams>,
    mut receiver: TReceiver,
) where
    TParams: AgentStreamingRequest,
    TReceiver: ReceivesAgentRelay<TParams::Response>,
{
    loop {
        match agent_relay.next().await {
            AgentRelayStep::Concluded(conclusion) => {
                return receiver.receive_conclusion(conclusion).await;
            }
            AgentRelayStep::Partial {
                generated_by,
                partial_result,
                relay,
            } => {
                receiver.receive_partial(generated_by, partial_result).await;
                agent_relay = relay;
            }
        }
    }
}

pub async fn forward_agent_relay<TParams, TReceiver>(
    agent_relay: AgentRelay<TParams>,
    connection_close: CancellationToken,
    receiver: TReceiver,
) where
    TParams: AgentStreamingRequest,
    TReceiver: ReceivesAgentRelay<TParams::Response>,
{
    let settlement = agent_relay.settlement();

    select! {
        biased;

        () = connection_close.cancelled() => {}
        () = relay_to_receiver(agent_relay, receiver) => {}
    }

    settlement.cancelled().await;
}
