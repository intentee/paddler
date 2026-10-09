use llama_cpp_bindings::token::LlamaToken;

#[derive(Debug)]
pub enum SamplingOutcome {
    AllCandidatesEliminated,
    GrammarRejectedModelOutput(String),
    Token(LlamaToken),
}
