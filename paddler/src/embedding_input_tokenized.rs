use llama_cpp_2::token::LlamaToken;

pub struct EmbeddingInputTokenized {
    pub id: String,
    pub llama_tokens: Vec<LlamaToken>,
}
