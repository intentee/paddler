use std::sync::Arc;

use llama_cpp_bindings::StreamingMarkers;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use tokio::sync::mpsc;

use crate::grammar_sampler::GrammarSampler;
use crate::prepared_prompt::PreparedPrompt;
use crate::slot_guard::SlotGuard;
use crate::tool_call_pipeline::ToolCallPipeline;

pub struct PreparedGenerationRequest {
    pub generate_tokens_stop_rx: mpsc::UnboundedReceiver<()>,
    pub generated_tokens_tx: mpsc::UnboundedSender<GeneratedTokenResult>,
    pub grammar_sampler: Option<GrammarSampler>,
    pub max_tokens: i32,
    pub prompt: PreparedPrompt,
    pub slot_guard: SlotGuard,
    pub streaming_markers: Arc<StreamingMarkers>,
    pub tool_call_pipeline: Option<ToolCallPipeline>,
}
