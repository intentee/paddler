use llama_cpp_bindings::token::LlamaToken;

pub struct EmbeddingInputTokenized {
    pub id: String,
    pub tokens: Vec<LlamaToken>,
}
