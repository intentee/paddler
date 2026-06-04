use std::sync::Arc;

use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use tokio::sync::mpsc;

use crate::from_request_params::FromRequestParams;
use crate::slot_aggregated_status::SlotAggregatedStatus;
use crate::slot_guard::SlotGuard;

pub struct GenerateEmbeddingBatchRequest {
    pub generate_embedding_stop_rx: mpsc::UnboundedReceiver<()>,
    pub generated_embedding_tx: mpsc::UnboundedSender<EmbeddingResult>,
    pub params: GenerateEmbeddingBatchParams,
    pub slot_guard: SlotGuard,
}

impl FromRequestParams for GenerateEmbeddingBatchRequest {
    type RequestParams = GenerateEmbeddingBatchParams;
    type Response = EmbeddingResult;

    fn from_request_params(
        params: Self::RequestParams,
        generated_embedding_tx: mpsc::UnboundedSender<Self::Response>,
        generate_embedding_stop_rx: mpsc::UnboundedReceiver<()>,
        slot_aggregated_status: Arc<SlotAggregatedStatus>,
    ) -> Self {
        Self {
            generate_embedding_stop_rx,
            generated_embedding_tx,
            params,
            slot_guard: SlotGuard::new(slot_aggregated_status),
        }
    }
}
