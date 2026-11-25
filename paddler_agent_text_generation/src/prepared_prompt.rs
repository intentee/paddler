use llama_cpp_model::llama_token::LlamaToken;

use crate::multimodal_prompt::MultimodalPrompt;

pub enum PreparedPrompt {
    Multimodal(MultimodalPrompt),
    TextTokens(Vec<LlamaToken>),
}
