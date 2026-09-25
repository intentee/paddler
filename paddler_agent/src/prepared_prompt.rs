use llama_cpp_bindings::token::LlamaToken;

use crate::prepared_multimodal_prompt::PreparedMultimodalPrompt;

pub enum PreparedPrompt {
    Multimodal(PreparedMultimodalPrompt),
    TextTokens(Vec<LlamaToken>),
}
