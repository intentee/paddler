use anyhow::Result;
use llama_cpp_bindings::llama_batch::LlamaBatch;

use crate::agent::continuous_batch_scheduler::contributions::Contributions;

pub struct BatchPass<'tokens> {
    pub batch: LlamaBatch<'tokens>,
    pub contributions: Contributions,
}

impl BatchPass<'_> {
    /// # Errors
    /// Forwards [`LlamaBatch::new`] failures verbatim.
    pub fn new(batch_n_tokens: usize, max_sequences: i32) -> Result<Self> {
        Ok(Self {
            batch: LlamaBatch::new(batch_n_tokens, max_sequences)?,
            contributions: Contributions::default(),
        })
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.contributions.is_empty()
    }
}
