use llama_cpp_bindings::context::LlamaContext;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SequenceContextLimit {
    Tokens(usize),
    Unlimited,
}

impl SequenceContextLimit {
    #[must_use]
    pub fn of(llama_context: &LlamaContext<'_>) -> Self {
        if llama_context.has_memory() {
            Self::Tokens(llama_context.n_ctx_seq() as usize)
        } else {
            Self::Unlimited
        }
    }
}
