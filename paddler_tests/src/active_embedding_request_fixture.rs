use std::num::NonZeroU16;
use std::sync::Arc;

use tokio::sync::mpsc;

use paddler_agent_embeddings::active_embedding_request::ActiveEmbeddingRequest;
use paddler_agent_embeddings::embedding_input_tokenized::EmbeddingInputTokenized;
use paddler_agent_embeddings::prepared_embedding_batch_request::PreparedEmbeddingBatchRequest;
use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_agent_status::slot_guard::SlotGuard;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::embedding_result::EmbeddingResult;

pub struct ActiveEmbeddingRequestFixture {
    pub admitted_requests: Vec<ActiveEmbeddingRequest>,
    pub generate_embedding_stop_tx: mpsc::UnboundedSender<()>,
    pub generated_embedding_rx: mpsc::UnboundedReceiver<EmbeddingResult>,
}

impl ActiveEmbeddingRequestFixture {
    #[must_use]
    pub fn new(
        inputs: Vec<EmbeddingInputTokenized>,
        slot_aggregated_status: Arc<SlotAggregatedStatus>,
        reserved_slots: NonZeroU16,
    ) -> Self {
        let (generated_embedding_tx, generated_embedding_rx) = mpsc::unbounded_channel();
        let (generate_embedding_stop_tx, generate_embedding_stop_rx) = mpsc::unbounded_channel();

        Self {
            admitted_requests: PreparedEmbeddingBatchRequest {
                generate_embedding_stop_rx,
                generated_embedding_tx,
                inputs,
                normalization_method: EmbeddingNormalizationMethod::None,
                unembeddable_documents: Vec::new(),
                slot_guard: SlotGuard::new(slot_aggregated_status, reserved_slots),
            }
            .admit(None)
            .into_iter()
            .collect(),
            generate_embedding_stop_tx,
            generated_embedding_rx,
        }
    }
}
