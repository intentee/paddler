use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::embedding_result::EmbeddingResult;
use tokio::sync::mpsc;

use crate::embedding_input_tokenized::EmbeddingInputTokenized;
use crate::slot_guard::SlotGuard;

pub struct PreparedEmbeddingBatchRequest {
    pub generate_embedding_stop_rx: mpsc::UnboundedReceiver<()>,
    pub generated_embedding_tx: mpsc::UnboundedSender<EmbeddingResult>,
    pub inputs: Vec<EmbeddingInputTokenized>,
    pub normalization_method: EmbeddingNormalizationMethod,
    pub slot_guard: SlotGuard,
}
