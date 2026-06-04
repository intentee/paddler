use anyhow::Result;
use llama_cpp_bindings::llama_batch::LlamaBatch;

use crate::continuous_batch_scheduler::contributions::Contributions;

pub struct BatchPass<'tokens> {
    pub batch: LlamaBatch<'tokens>,
    pub contributions: Contributions,
}

impl BatchPass<'_> {
    /// # Errors
    /// Forwards [`LlamaBatch::new`] failures verbatim.
    pub fn new(n_batch: usize, max_sequences: i32) -> Result<Self> {
        Ok(Self {
            batch: LlamaBatch::new(n_batch, max_sequences)?,
            contributions: Contributions::default(),
        })
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.contributions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::BatchPass;

    #[test]
    fn new_creates_empty_batch_pass() {
        let batch_pass = BatchPass::new(16, 1).unwrap();

        assert_eq!(batch_pass.batch.n_tokens(), 0);
        assert!(batch_pass.is_empty());
    }

    #[test]
    fn new_forwards_llama_batch_error_for_oversized_n_batch() {
        let result = BatchPass::new(usize::MAX, 1);

        let error = result.err().unwrap();

        assert!(error.to_string().contains("overflow"));
    }
}
