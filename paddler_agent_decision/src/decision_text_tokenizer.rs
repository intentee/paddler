use icu_normalizer::ComposingNormalizerBorrowed;
use llama_cpp_bindings::model::AddBos;
use llama_cpp_bindings::model::ParseSpecialTokens;
use llama_cpp_bindings::token::LlamaToken;

use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;

use crate::decision_error::DecisionError;

pub struct DecisionTextTokenizer {
    pub loaded_llama_model: LoadedLlamaModel,
}

impl DecisionTextTokenizer {
    pub fn tokenize(&self, text: &str) -> Result<Vec<LlamaToken>, DecisionError> {
        self.loaded_llama_model
            .model
            .str_to_token(
                &ComposingNormalizerBorrowed::new_nfc().normalize(text),
                AddBos::Never,
                ParseSpecialTokens::Never,
            )
            .map_err(DecisionError::InputTokenizationFailed)
    }
}
