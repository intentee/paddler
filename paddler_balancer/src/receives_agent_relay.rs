use std::future::Future;

use paddler_messaging::streamable_result::StreamableResult;

use crate::agent_relay_conclusion::AgentRelayConclusion;

pub trait ReceivesAgentRelay<TResponse>
where
    TResponse: StreamableResult,
{
    fn receive_conclusion(
        self,
        conclusion: AgentRelayConclusion<TResponse::Final>,
    ) -> impl Future<Output = ()> + Send;

    fn receive_partial(
        &mut self,
        generated_by: Option<String>,
        partial_result: TResponse::Partial,
    ) -> impl Future<Output = ()> + Send;
}
