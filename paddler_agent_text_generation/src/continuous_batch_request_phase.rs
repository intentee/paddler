use crate::continuous_batch_generation_step::ContinuousBatchGenerationStep;
use crate::continuous_batch_terminal_outcome::ContinuousBatchTerminalOutcome;
use crate::multimodal_prompt_ingestion::MultimodalPromptIngestion;

#[derive(Debug)]
pub enum ContinuousBatchRequestPhase {
    IngestingText,
    IngestingLeadingBos(MultimodalPromptIngestion),
    IngestingMultimodal(MultimodalPromptIngestion),
    Generating(ContinuousBatchGenerationStep),
    Completed(ContinuousBatchTerminalOutcome),
}

impl ContinuousBatchRequestPhase {
    #[must_use]
    pub fn after_its_text_prompt(self, last_batch_position: i32) -> Self {
        match self {
            Self::IngestingLeadingBos(ingestion) => Self::IngestingMultimodal(ingestion),
            Self::IngestingText
            | Self::IngestingMultimodal(_)
            | Self::Generating(_)
            | Self::Completed(_) => {
                Self::Generating(ContinuousBatchGenerationStep::ReadyToSample {
                    batch_position: last_batch_position,
                })
            }
        }
    }

    #[must_use]
    pub const fn samples_after_its_text_prompt(&self) -> bool {
        matches!(self, Self::IngestingText)
    }
}
