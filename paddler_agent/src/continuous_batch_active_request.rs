use llama_cpp_bindings::SampledTokenClassifier;
use llama_cpp_bindings::sampling::LlamaSampler;
use log::warn;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TryRecvError;

use crate::continuous_batch_request_state::ContinuousBatchRequestState;
use crate::slot_guard::SlotGuard;
use crate::tool_call_pipeline::ToolCallPipeline;

fn send_outcome_or_warn(
    agent_name: Option<&str>,
    sequence_id: u16,
    generated_tokens_tx: &mpsc::UnboundedSender<GeneratedTokenResult>,
    outcome: GeneratedTokenResult,
) {
    if generated_tokens_tx.send(outcome).is_err() {
        warn!(
            "{agent_name:?}: sequence {sequence_id} failed to send result to client (receiver dropped)"
        );
    }
}

pub struct ContinuousBatchActiveRequest {
    pub state: ContinuousBatchRequestState,
    pub chain: LlamaSampler,
    pub token_classifier: SampledTokenClassifier<'static>,
    pub grammar_sampler: Option<LlamaSampler>,
    pub generated_tokens_tx: mpsc::UnboundedSender<GeneratedTokenResult>,
    pub generate_tokens_stop_rx: mpsc::UnboundedReceiver<()>,
    pub slot_guard: SlotGuard,
    pub tool_call_pipeline: Option<ToolCallPipeline>,
}

impl ContinuousBatchActiveRequest {
    pub fn complete_with_outcome(
        &mut self,
        agent_name: Option<&str>,
        outcome: GeneratedTokenResult,
    ) {
        send_outcome_or_warn(
            agent_name,
            self.state.sequence_id,
            &self.generated_tokens_tx,
            outcome,
        );

        self.state.mark_completed();
    }

    pub fn is_stop_requested(&mut self) -> bool {
        match self.generate_tokens_stop_rx.try_recv() {
            Ok(()) | Err(TryRecvError::Disconnected) => true,
            Err(TryRecvError::Empty) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use log::LevelFilter;
    use tokio::sync::mpsc;

    use super::send_outcome_or_warn;
    use paddler_messaging::generated_token_result::GeneratedTokenResult;

    #[test]
    fn delivers_outcome_to_a_live_receiver() {
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();

        send_outcome_or_warn(
            Some("agent"),
            7,
            &generated_tokens_tx,
            GeneratedTokenResult::ContentToken("hello".to_owned()),
        );

        assert!(matches!(
            generated_tokens_rx.try_recv(),
            Ok(GeneratedTokenResult::ContentToken(token)) if token == "hello"
        ));
    }

    #[test]
    fn warns_without_panicking_when_the_receiver_was_dropped() {
        log::set_max_level(LevelFilter::Trace);

        let (generated_tokens_tx, generated_tokens_rx) = mpsc::unbounded_channel();

        drop(generated_tokens_rx);

        send_outcome_or_warn(
            None,
            42,
            &generated_tokens_tx,
            GeneratedTokenResult::ContentToken("dropped".to_owned()),
        );

        assert!(generated_tokens_tx.is_closed());
    }
}
