use llama_cpp_bindings::sampling::LlamaSampler;
use llama_cpp_bindings::token::LlamaToken;
use paddler_types::generated_token_result::GeneratedTokenResult;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TryRecvError;

use crate::agent::continuous_batch_request_phase::ContinuousBatchRequestPhase;

pub struct ContinuousBatchActiveRequest {
    pub chain: LlamaSampler,
    pub current_token_position: i32,
    pub grammar_sampler: Option<LlamaSampler>,
    pub generated_tokens_count: i32,
    pub generated_tokens_tx: mpsc::UnboundedSender<GeneratedTokenResult>,
    pub generate_tokens_stop_rx: mpsc::UnboundedReceiver<()>,
    pub i_batch: Option<i32>,
    pub max_tokens: i32,
    pub pending_sampled_token: Option<LlamaToken>,
    pub phase: ContinuousBatchRequestPhase,
    pub prompt_tokens: Vec<LlamaToken>,
    pub prompt_tokens_ingested: usize,
    pub sequence_id: i32,
    pub utf8_decoder: encoding_rs::Decoder,
}

impl ContinuousBatchActiveRequest {
    pub fn is_stop_requested(&mut self) -> bool {
        match self.generate_tokens_stop_rx.try_recv() {
            Ok(()) | Err(TryRecvError::Disconnected) => true,
            Err(TryRecvError::Empty) => false,
        }
    }

    #[must_use]
    pub fn remaining_prompt_tokens(&self) -> &[LlamaToken] {
        &self.prompt_tokens[self.prompt_tokens_ingested..]
    }
}
