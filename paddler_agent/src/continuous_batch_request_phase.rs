use crate::continuous_batch_terminal_outcome::ContinuousBatchTerminalOutcome;

#[derive(Debug)]
pub enum ContinuousBatchRequestPhase {
    Ingesting,
    Generating,
    Completed(ContinuousBatchTerminalOutcome),
}
