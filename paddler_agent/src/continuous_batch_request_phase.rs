use crate::continuous_batch_generating_state::ContinuousBatchGeneratingState;
use crate::continuous_batch_ingesting_state::ContinuousBatchIngestingState;
use crate::continuous_batch_terminal_outcome::ContinuousBatchTerminalOutcome;

#[derive(Debug)]
pub enum ContinuousBatchRequestPhase {
    Ingesting(ContinuousBatchIngestingState),
    Generating(ContinuousBatchGeneratingState),
    Completed(ContinuousBatchTerminalOutcome),
}
