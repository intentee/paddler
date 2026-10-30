use llama_cpp_bindings::token::LlamaToken;

#[derive(Debug, Eq, PartialEq)]
pub struct EmbeddingInputTokenized {
    pub id: String,
    pub tokens: Vec<LlamaToken>,
}
