use std::sync::Arc;

use llama_cpp_bindings::model::AddBos;
use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::model::ParseSpecialTokens;
use llama_cpp_bindings::token::LlamaToken;

use paddler_messaging::chat_template_message::ChatTemplateMessage;
use paddler_messaging::chat_template_message_content::ChatTemplateMessageContent;

use crate::chat_prompt_render_request::ChatPromptRenderRequest;
use crate::chat_prompt_renderer::ChatPromptRenderer;
use crate::model_bos_token::ModelBosToken;
use crate::require_prompt_fits_sequence_context::require_prompt_fits_sequence_context;
use crate::text_generation_error::TextGenerationError;

const LEADING_BOS_PROBE_TEXT: &str = "hi";

pub struct PromptTokenizer {
    pub model: Arc<LlamaModel>,
    pub model_bos_token: ModelBosToken,
    pub sequence_context_size: u32,
}

impl PromptTokenizer {
    pub fn template_renders_its_own_bos_token(
        &self,
        chat_prompt_renderer: &ChatPromptRenderer,
    ) -> Result<bool, TextGenerationError> {
        let probe_prompt = chat_prompt_renderer.render(ChatPromptRenderRequest {
            add_generation_prompt: true,
            enable_thinking: false,
            messages: &[ChatTemplateMessage {
                content: ChatTemplateMessageContent::Text(LEADING_BOS_PROBE_TEXT.to_owned()),
                role: "user".to_owned(),
            }],
            tools: &[],
        })?;

        self.tokens_with_special_tokens(&probe_prompt)
            .map(|probe_tokens| {
                self.model_bos_token
                    .is_duplicated_at_start_of(&probe_tokens)
            })
    }

    pub fn tokenize(&self, prompt: &str) -> Result<Vec<LlamaToken>, TextGenerationError> {
        let mut prompt_tokens = self.tokens_with_special_tokens(prompt)?;

        if self
            .model_bos_token
            .is_duplicated_at_start_of(&prompt_tokens)
        {
            prompt_tokens.remove(0);
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
