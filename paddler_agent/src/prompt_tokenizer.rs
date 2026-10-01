use std::sync::Arc;

use llama_cpp_bindings::model::AddBos;
use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::token::LlamaToken;

use crate::generation_request_rejection::GenerationRequestRejection;
use crate::model_bos_token::ModelBosToken;
use crate::require_prompt_fits_sequence_context::require_prompt_fits_sequence_context;

pub struct PromptTokenizer {
    pub model: Arc<LlamaModel>,
    pub sequence_context_size: u32,
}

impl PromptTokenizer {
    pub fn renders_its_own_bos_token(
        &self,
        prompt: &str,
    ) -> Result<bool, GenerationRequestRejection> {
        self.tokens_with_special_tokens(prompt)
            .map(|prompt_tokens| {
                ModelBosToken::of(&self.model).is_duplicated_at_start_of(&prompt_tokens)
            })
    }

    pub fn tokenize(&self, prompt: &str) -> Result<Vec<LlamaToken>, GenerationRequestRejection> {
        let mut prompt_tokens = self.tokens_with_special_tokens(prompt)?;

        if ModelBosToken::of(&self.model).is_duplicated_at_start_of(&prompt_tokens) {
            prompt_tokens.remove(0);
        }

        require_prompt_fits_sequence_context(prompt_tokens.len(), self.sequence_context_size)?;

        Ok(prompt_tokens)
    }

    fn tokens_with_special_tokens(
        &self,
        prompt: &str,
    ) -> Result<Vec<LlamaToken>, GenerationRequestRejection> {
        self.model
            .str_to_token(prompt, AddBos::Always)
            .map_err(GenerationRequestRejection::PromptTokenizationFailed)
    }
}
