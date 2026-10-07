use llama_cpp_bindings::token::LlamaToken;

use crate::multimodal_prompt_ingestion::MultimodalPromptIngestion;

pub enum PreparedPrompt {
    Multimodal(MultimodalPromptIngestion),
    TextTokens(Vec<LlamaToken>),
}
