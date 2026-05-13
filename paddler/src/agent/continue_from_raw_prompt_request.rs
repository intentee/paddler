use std::sync::Arc;

use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::request_params::ContinueFromRawPromptParams;
use tokio::sync::mpsc;

use crate::agent::from_request_params::FromRequestParams;
use crate::agent::slot_guard::SlotGuard;
use crate::slot_aggregated_status::SlotAggregatedStatus;

pub struct ContinueFromRawPromptRequest {
    pub generate_tokens_stop_rx: mpsc::UnboundedReceiver<()>,
    pub generated_tokens_tx: mpsc::UnboundedSender<GeneratedTokenResult>,
    pub params: ContinueFromRawPromptParams,
    pub slot_guard: SlotGuard,
}

impl FromRequestParams for ContinueFromRawPromptRequest {
    type RequestParams = ContinueFromRawPromptParams;
    type Response = GeneratedTokenResult;

    fn from_request_params(
        params: Self::RequestParams,
        generated_tokens_tx: mpsc::UnboundedSender<Self::Response>,
        generate_tokens_stop_rx: mpsc::UnboundedReceiver<()>,
        slot_aggregated_status: Arc<SlotAggregatedStatus>,
    ) -> Self {
        Self {
            generate_tokens_stop_rx,
            generated_tokens_tx,
            params,
            slot_guard: SlotGuard::new(slot_aggregated_status),
        }
    }
}
