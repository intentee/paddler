use std::num::NonZeroU32;

use paddler_messaging::generated_token_result::GeneratedTokenResult;
use tokio::sync::mpsc;

use crate::prepared_prompt::PreparedPrompt;
use crate::slot_guard::SlotGuard;
use crate::token_classification::TokenClassification;
use crate::token_sampling::TokenSampling;
use crate::tool_call_handling::ToolCallHandling;

pub struct PreparedGenerationRequest {
    pub generate_tokens_stop_rx: mpsc::UnboundedReceiver<()>,
    pub generated_tokens_tx: mpsc::UnboundedSender<GeneratedTokenResult>,
    pub max_tokens: NonZeroU32,
    pub prompt: PreparedPrompt,
    pub slot_guard: SlotGuard,
    pub token_classification: TokenClassification,
    pub token_sampling: TokenSampling,
    pub tool_call_handling: ToolCallHandling,
}
