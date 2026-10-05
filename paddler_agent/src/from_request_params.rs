use std::sync::Arc;

use tokio::sync::mpsc;

use crate::slot_aggregated_status::SlotAggregatedStatus;
use paddler_messaging::management_socket::agent::response::Response;

pub trait FromRequestParams: Send + Sync {
    type RequestParams;
    type Response: Into<Response>;

    fn from_request_params(
        params: Self::RequestParams,
        response_tx: mpsc::UnboundedSender<Self::Response>,
        stop_rx: mpsc::UnboundedReceiver<()>,
        slot_aggregated_status: Arc<SlotAggregatedStatus>,
    ) -> Self;
}
