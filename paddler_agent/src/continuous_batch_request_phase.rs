use crate::continuous_batch_terminal_outcome::ContinuousBatchTerminalOutcome;
use crate::multimodal_prompt_ingestion::MultimodalPromptIngestion;

#[derive(Debug)]
pub enum ContinuousBatchRequestPhase {
    IngestingText,
    IngestingMultimodal(MultimodalPromptIngestion),
    Generating,
    Completed(ContinuousBatchTerminalOutcome),
}
