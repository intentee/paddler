use llama_cpp_bindings::SampledTokenClassifier;
use llama_cpp_bindings::sampling::LlamaSampler;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TryRecvError;

use crate::continuous_batch_request_state::ContinuousBatchRequestState;
use crate::continuous_batch_terminal_delivery::ContinuousBatchTerminalDelivery;
use crate::continuous_batch_terminal_outcome::ContinuousBatchTerminalOutcome;
use crate::sequence_id_guard::SequenceIdGuard;
use crate::slot_guard::SlotGuard;
use crate::tool_call_pipeline::ToolCallPipeline;

pub struct ContinuousBatchActiveRequest {
    pub state: ContinuousBatchRequestState,
    pub chain: LlamaSampler,
    pub token_classifier: SampledTokenClassifier<'static>,
    pub grammar_sampler: Option<LlamaSampler>,
    pub generated_tokens_tx: mpsc::UnboundedSender<GeneratedTokenResult>,
    pub generate_tokens_stop_rx: mpsc::UnboundedReceiver<()>,
    pub sequence_id_guard: SequenceIdGuard,
    pub slot_guard: SlotGuard,
    pub tool_call_pipeline: Option<ToolCallPipeline>,
}

impl ContinuousBatchActiveRequest {
    pub fn complete_with_outcome(&mut self, outcome: GeneratedTokenResult) {
        self.state
            .mark_completed(ContinuousBatchTerminalOutcome::EmitToClient(outcome));
    }

    #[must_use]
    pub fn into_terminal_delivery(self) -> ContinuousBatchTerminalDelivery {
        ContinuousBatchTerminalDelivery::new(
            self.generated_tokens_tx,
            self.sequence_id_guard,
            self.state.into_terminal_outcome(),
        )
    }

    pub fn is_stop_requested(&mut self) -> bool {
        match self.generate_tokens_stop_rx.try_recv() {
            Ok(()) | Err(TryRecvError::Disconnected) => true,
            Err(TryRecvError::Empty) => false,
        }
    }
}
