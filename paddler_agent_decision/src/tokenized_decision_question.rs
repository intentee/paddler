use llama_cpp_model::llama_token::LlamaToken;

pub struct TokenizedDecisionQuestion {
    pub id: String,
    pub instructions: Vec<LlamaToken>,
    pub options: Vec<Vec<LlamaToken>>,
}
