use llama_cpp_bindings::token::LlamaToken;

pub struct TokenizedDecisionQuestion {
    pub id: String,
    pub instructions: Vec<LlamaToken>,
    pub options: Vec<Vec<LlamaToken>>,
}
