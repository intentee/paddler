use paddler_messaging::generated_token_result::GeneratedTokenResult;
use tokio::sync::mpsc;

use crate::continuous_batch_terminal_outcome::ContinuousBatchTerminalOutcome;
use crate::send_generated_token_result_or_warn::send_generated_token_result_or_warn;
use crate::sequence_id_guard::SequenceIdGuard;

pub struct ContinuousBatchTerminalDelivery {
    generated_tokens_tx: mpsc::UnboundedSender<GeneratedTokenResult>,
    _sequence_id_guard: SequenceIdGuard,
    terminal_outcome: ContinuousBatchTerminalOutcome,
}

impl ContinuousBatchTerminalDelivery {
    #[must_use]
    pub const fn new(
        generated_tokens_tx: mpsc::UnboundedSender<GeneratedTokenResult>,
        sequence_id_guard: SequenceIdGuard,
        terminal_outcome: ContinuousBatchTerminalOutcome,
    ) -> Self {
        Self {
            generated_tokens_tx,
            _sequence_id_guard: sequence_id_guard,
            terminal_outcome,
        }
    }

    pub fn deliver(self, agent_name: Option<&str>) {
        match self.terminal_outcome {
            ContinuousBatchTerminalOutcome::EmitNothing => {}
            ContinuousBatchTerminalOutcome::EmitToClient(terminal_result) => {
                send_generated_token_result_or_warn(
                    agent_name,
                    &self.generated_tokens_tx,
                    terminal_result,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use tokio::sync::mpsc;

    use super::ContinuousBatchTerminalDelivery;
    use crate::continuous_batch_terminal_outcome::ContinuousBatchTerminalOutcome;
    use crate::sequence_id_guard::SequenceIdGuard;
    use crate::sequence_id_pool::SequenceIdPool;

    #[test]
    fn delivering_an_emit_to_client_outcome_sends_the_terminal_result() {
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();

        ContinuousBatchTerminalDelivery::new(
            generated_tokens_tx,
            SequenceIdGuard::acquire(&SequenceIdPool::new(1)).unwrap(),
            ContinuousBatchTerminalOutcome::EmitToClient(GeneratedTokenResult::SamplerError(
                "stopped".to_owned(),
            )),
        )
        .deliver(Some("agent"));

        assert!(matches!(
            generated_tokens_rx.try_recv(),
            Ok(GeneratedTokenResult::SamplerError(message)) if message == "stopped"
        ));
    }

    #[test]
    fn delivering_an_emit_nothing_outcome_sends_no_terminal_result() {
        let (generated_tokens_tx, mut generated_tokens_rx) =
            mpsc::unbounded_channel::<GeneratedTokenResult>();

        ContinuousBatchTerminalDelivery::new(
            generated_tokens_tx,
            SequenceIdGuard::acquire(&SequenceIdPool::new(1)).unwrap(),
            ContinuousBatchTerminalOutcome::EmitNothing,
        )
        .deliver(None);

        assert!(generated_tokens_rx.try_recv().is_err());
    }
}
