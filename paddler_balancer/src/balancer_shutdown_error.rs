use paddler_messaging::jsonrpc::error::Error as JsonRpcError;

#[must_use]
pub fn balancer_shutdown_error() -> JsonRpcError {
    JsonRpcError {
        code: 503,
        description: "balancer is shutting down".to_owned(),
    }
}
