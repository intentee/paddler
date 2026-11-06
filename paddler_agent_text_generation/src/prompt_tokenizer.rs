use std::sync::Arc;

use llama_cpp_bindings::chat_session::ChatSession;
use llama_cpp_bindings::model::add_bos::AddBos;
use llama_cpp_bindings::model::parse_special_tokens::ParseSpecialTokens;
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
        let mut prompt_tokens = self
            .model
            .str_to_token(prompt, AddBos::Always, ParseSpecialTokens::Always)
            .map_err(TextGenerationError::PromptTokenizationFailed)?;

        if self
            .model_bos_token
            .is_duplicated_at_start_of(&prompt_tokens)
        {
            prompt_tokens.remove(0);
        }

        self.servable(prompt_tokens)
    }

    pub fn tokenize_chat_session(
        &self,
        chat_session: &ChatSession,
    ) -> Result<Vec<LlamaToken>, TextGenerationError> {
        self.servable(
            chat_session
                .tokenize_prompt(&self.model)
                .map_err(TextGenerationError::ChatPromptTokenizationFailed)?,
        )
    }

    fn servable(
        &self,
        prompt_tokens: Vec<LlamaToken>,
    ) -> Result<Vec<LlamaToken>, TextGenerationError> {
        if prompt_tokens.is_empty() {
            return Err(TextGenerationError::PromptTokenizedToNothing);
        }

        require_prompt_fits_sequence_context(prompt_tokens.len(), self.sequence_context_size)?;

        Ok(prompt_tokens)
    }
}
