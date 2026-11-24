use llama_cpp_model::llama_token::LlamaToken;

#[derive(Debug)]
pub enum SamplingOutcome {
    AllCandidatesEliminated,
    GrammarRejectedModelOutput(String),
    Token(LlamaToken),
}
