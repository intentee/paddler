use std::num::NonZeroU16;
use std::num::NonZeroU32;
use std::sync::Arc;

use tokio::sync::mpsc;

use paddler_agent_runtime::agent_request::AgentRequest;
use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_agent_status::slot_guard::SlotGuard;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;

pub struct RawPromptGeneration {
    pub agent_request: AgentRequest<ContinueFromRawPromptParams, GeneratedTokenResult>,
    pub generate_tokens_stop_tx: mpsc::UnboundedSender<()>,
    pub generated_tokens_rx: mpsc::UnboundedReceiver<GeneratedTokenResult>,
}

impl RawPromptGeneration {
    #[must_use]
    pub fn new(raw_prompt: &str, slot_aggregated_status: Arc<SlotAggregatedStatus>) -> Self {
        let (generated_tokens_tx, generated_tokens_rx) = mpsc::unbounded_channel();
        let (generate_tokens_stop_tx, generate_tokens_stop_rx) = mpsc::unbounded_channel();

        Self {
            agent_request: AgentRequest {
                params: ContinueFromRawPromptParams {
                    grammar: None,
                    max_tokens: NonZeroU32::MIN,
                    raw_prompt: raw_prompt.to_owned(),
                },
                response_tx: generated_tokens_tx,
                slot_guard: SlotGuard::new(slot_aggregated_status, NonZeroU16::MIN),
                stop_rx: generate_tokens_stop_rx,
            },
            generate_tokens_stop_tx,
            generated_tokens_rx,
        }
    }
}
