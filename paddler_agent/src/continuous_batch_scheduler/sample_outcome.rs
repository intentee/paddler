use llama_cpp_bindings::token::LlamaToken;

pub enum SampleOutcome {
    Sampled(LlamaToken),
    AllCandidatesEliminated,
    GrammarRejected(String),
    Failed(String),
}
