use llama_cpp_bindings::token::LlamaToken;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LeadingBosIngestion {
    Ingest(LlamaToken),
    Skip,
}

impl LeadingBosIngestion {
    #[must_use]
    pub const fn token_count(self) -> usize {
        match self {
            Self::Ingest(_) => 1,
            Self::Skip => 0,
        }
    }
}
