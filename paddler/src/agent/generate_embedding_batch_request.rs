use std::sync::Arc;

use crate::embedding_result::EmbeddingResult;
use crate::request_params::GenerateEmbeddingBatchParams;
use tokio::sync::mpsc;

use crate::agent::from_request_params::FromRequestParams;
use crate::agent::slot_guard::SlotGuard;
use crate::slot_aggregated_status::SlotAggregatedStatus;

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
