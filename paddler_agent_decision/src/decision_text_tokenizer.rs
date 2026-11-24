use std::ops::Deref;
use std::sync::Arc;

use icu_normalizer::ComposingNormalizerBorrowed;
use llama_cpp_model::add_bos::AddBos;
use llama_cpp_model::llama_model::LlamaModel;
use llama_cpp_model::llama_token::LlamaToken;
use llama_cpp_model::parse_special_tokens::ParseSpecialTokens;

use crate::decision_error::DecisionError;
use crate::escape_special_token_lookalikes::escape_special_token_lookalikes;

pub struct DecisionTextTokenizer {
    pub model: Arc<dyn Deref<Target = LlamaModel> + Send + Sync>,
}

impl DecisionTextTokenizer {
    /// # Errors
    ///
    /// Returns [`DecisionError::InputTokenizationFailed`] when the model cannot tokenize the normalized text.
    pub fn tokenize(&self, text: &str) -> Result<Vec<LlamaToken>, DecisionError> {
        self.model
            .str_to_token(
                &ComposingNormalizerBorrowed::new_nfc()
                    .normalize(&escape_special_token_lookalikes(text)),
                AddBos::Never,
                ParseSpecialTokens::Never,
            )
            .map_err(DecisionError::InputTokenizationFailed)
    }
}
