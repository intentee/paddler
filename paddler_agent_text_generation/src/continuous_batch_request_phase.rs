use crate::continuous_batch_generation_step::ContinuousBatchGenerationStep;
use crate::continuous_batch_terminal_outcome::ContinuousBatchTerminalOutcome;
use crate::multimodal_prompt::MultimodalPrompt;

#[derive(Debug)]
pub enum ContinuousBatchRequestPhase {
    IngestingText,
    IngestingMultimodal(MultimodalPrompt),
    Generating(ContinuousBatchGenerationStep),
    Completed(ContinuousBatchTerminalOutcome),
}
