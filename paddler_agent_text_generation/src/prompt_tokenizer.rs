use std::sync::Arc;

use llama_cpp_bindings::model::AddBos;
use llama_cpp_bindings::model::ParseSpecialTokens;
use llama_cpp_bindings::token::LlamaToken;

use paddler_agent_runtime::resident_llama_model::ResidentLlamaModel;

use crate::model_bos_token::ModelBosToken;
use crate::require_prompt_fits_sequence_context::require_prompt_fits_sequence_context;
use crate::text_generation_error::TextGenerationError;

pub struct PromptTokenizer {
    pub model: Arc<ResidentLlamaModel>,
    pub model_bos_token: ModelBosToken,
    pub sequence_context_size: u32,
}

impl PromptTokenizer {
    pub fn tokenize(&self, prompt: &str) -> Result<Vec<LlamaToken>, TextGenerationError> {
        let mut prompt_tokens = self.tokens_with_special_tokens(prompt)?;

        if self
            .model_bos_token
            .is_duplicated_at_start_of(&prompt_tokens)
        {
            prompt_tokens.remove(0);
        }

        if prompt_tokens.is_empty() {
            return Err(TextGenerationError::PromptTokenizedToNothing);
        }

        require_prompt_fits_sequence_context(prompt_tokens.len(), self.sequence_context_size)?;

        Ok(prompt_tokens)
    }

    fn tokens_with_special_tokens(
        &self,
        prompt: &str,
    ) -> Result<Vec<LlamaToken>, TextGenerationError> {
        self.model
            .str_to_token(prompt, AddBos::Always, ParseSpecialTokens::Always)
            .map_err(TextGenerationError::PromptTokenizationFailed)
    }
}
