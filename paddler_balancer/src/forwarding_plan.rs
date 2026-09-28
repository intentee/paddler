use paddler_messaging::jsonrpc::error::Error as JsonRpcError;

#[derive(Debug, Eq, PartialEq)]
pub enum ForwardingPlan<TResponse> {
    Finish,
    ForwardResponse { is_done: bool, response: TResponse },
    IgnoreDrainedResponse,
    ReplyWithErrorThenFinish(JsonRpcError),
    ReplyWithErrorThenStopAgent(JsonRpcError),
    ReplyWithErrorThenStopAgentThenFinish(JsonRpcError),
    StopAgent,
}
