use paddler_messaging::jsonrpc::error::Error as JsonRpcError;

#[derive(Debug, Eq, PartialEq)]
pub enum AgentResultStreamEvent<TResult> {
    Result { request_id: String, result: TResult },
    WireError(JsonRpcError),
}
