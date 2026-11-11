use std::collections::VecDeque;

use tokio::sync::mpsc;

use paddler_agent_runtime::send_result_or_warn::send_result_or_warn;
use paddler_agent_status::slot_guard::SlotGuard;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::embedding_result::EmbeddingResult;

use crate::embedding_input_tokenized::EmbeddingInputTokenized;

pub struct ActiveEmbeddingRequest {
    pub emitted_an_embedding: bool,
    pub generate_embedding_stop_rx: mpsc::UnboundedReceiver<()>,
    pub generated_embedding_tx: mpsc::UnboundedSender<EmbeddingResult>,
    pub normalization_method: EmbeddingNormalizationMethod,
    pub pending_inputs: VecDeque<EmbeddingInputTokenized>,
    pub slot_guard: SlotGuard,
}

impl ActiveEmbeddingRequest {
    pub fn finish(self, agent_name: Option<&str>) {
        send_result_or_warn(
            agent_name,
            &self.generated_embedding_tx,
            if self.emitted_an_embedding {
                EmbeddingResult::Done
            } else {
                EmbeddingResult::NoEmbeddingsProduced
            },
        );
    }

    #[must_use]
    pub const fn sequences(&self) -> u16 {
        self.slot_guard.slots().get()
    }
}
