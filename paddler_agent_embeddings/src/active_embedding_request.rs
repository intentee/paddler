use std::collections::VecDeque;

use tokio::sync::mpsc;

use paddler_agent_runtime::receives_stop_request::ReceivesStopRequest as _;
use paddler_agent_runtime::send_result_or_warn::send_result_or_warn;
use paddler_agent_status::slot_guard::SlotGuard;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::embedding_result::EmbeddingResult;

use crate::embedding_input_tokenized::EmbeddingInputTokenized;
use crate::embedding_request_continuation::EmbeddingRequestContinuation;

pub struct ActiveEmbeddingRequest {
    pub embeddings_emitted: usize,
    pub generate_embedding_stop_rx: mpsc::UnboundedReceiver<()>,
    pub generated_embedding_tx: mpsc::UnboundedSender<EmbeddingResult>,
    pub normalization_method: EmbeddingNormalizationMethod,
    pub pending_inputs: VecDeque<EmbeddingInputTokenized>,
    pub slot_guard: SlotGuard,
}

impl ActiveEmbeddingRequest {
    #[must_use]
    pub const fn sequences(&self) -> u16 {
        self.slot_guard.slots().get()
    }

    #[must_use]
    pub fn continue_unless_stop_requested(
        mut self,
        agent_name: Option<&str>,
    ) -> EmbeddingRequestContinuation {
        if self.generate_embedding_stop_rx.is_stop_requested() {
            self.finish(agent_name);

            return EmbeddingRequestContinuation::Concluded;
        }

        EmbeddingRequestContinuation::Continues(self)
    }

    #[must_use]
    pub fn continue_unless_every_input_is_embedded(
        self,
        agent_name: Option<&str>,
    ) -> EmbeddingRequestContinuation {
        if self.pending_inputs.is_empty() {
            self.finish(agent_name);

            return EmbeddingRequestContinuation::Concluded;
        }

        EmbeddingRequestContinuation::Continues(self)
    }

    fn finish(self, agent_name: Option<&str>) {
        send_result_or_warn(
            agent_name,
            &self.generated_embedding_tx,
            if self.embeddings_emitted == 0 {
                EmbeddingResult::NoEmbeddingsProduced
            } else {
                EmbeddingResult::Done
            },
        );
    }
}
