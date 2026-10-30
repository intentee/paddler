use std::ops::Deref;
use std::sync::Arc;

use llama_cpp_bindings::model::AddBos;
use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::model::ParseSpecialTokens;
use llama_cpp_bindings::token::LlamaToken;

use paddler_agent_pointer_head::pointer_head_input_normalization::PointerHeadInputNormalization;

use crate::decision_error::DecisionError;
use crate::escape_special_token_lookalikes::escape_special_token_lookalikes;

pub struct DecisionTextTokenizer {
    pub input_normalization: PointerHeadInputNormalization,
    pub model: Arc<dyn Deref<Target = LlamaModel> + Send + Sync>,
}

impl DecisionTextTokenizer {
    pub fn tokenize(&self, text: &str) -> Result<Vec<LlamaToken>, DecisionError> {
        self.model
            .str_to_token(
                &self
                    .input_normalization
                    .normalize(&escape_special_token_lookalikes(text)),
                AddBos::Never,
                ParseSpecialTokens::Never,
            )
            .map_err(DecisionError::InputTokenizationFailed)
    }
}
