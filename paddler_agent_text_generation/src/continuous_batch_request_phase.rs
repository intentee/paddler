use crate::continuous_batch_generation_step::ContinuousBatchGenerationStep;
use crate::continuous_batch_terminal_outcome::ContinuousBatchTerminalOutcome;
use crate::multimodal_prompt_ingestion::MultimodalPromptIngestion;

#[derive(Debug)]
pub enum ContinuousBatchRequestPhase {
    IngestingText,
    IngestingMultimodal(MultimodalPromptIngestion),
    Generating(ContinuousBatchGenerationStep),
    Completed(ContinuousBatchTerminalOutcome),
}
