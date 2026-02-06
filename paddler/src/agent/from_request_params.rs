use tokio::sync::mpsc;

use crate::agent::jsonrpc::response::Response;

pub trait FromRequestParams: Send + Sync {
    type RequestParams;
    type Response: Into<Response>;

    fn from_request_params(
        params: Self::RequestParams,
        response_tx: mpsc::UnboundedSender<Self::Response>,
        stop_rx: mpsc::UnboundedReceiver<()>,
    ) -> Self;
}
