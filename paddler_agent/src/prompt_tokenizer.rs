use std::sync::Arc;

use llama_cpp_bindings::model::AddBos;
use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::token::LlamaToken;

use crate::generation_request_rejection::GenerationRequestRejection;
use crate::require_prompt_fits_sequence_context::require_prompt_fits_sequence_context;

pub struct PromptTokenizer {
    pub model: Arc<LlamaModel>,
    pub sequence_context_size: u32,
}

impl PromptTokenizer {
    pub fn tokenize(&self, prompt: &str) -> Result<Vec<LlamaToken>, GenerationRequestRejection> {
        let prompt_tokens = self
            .model
            .str_to_token(prompt, AddBos::Always)
            .map_err(GenerationRequestRejection::PromptTokenizationFailed)?;

        require_prompt_fits_sequence_context(prompt_tokens.len(), self.sequence_context_size)?;

        Ok(prompt_tokens)
    }
}
