use llama_cpp_bindings::llama_batch::LlamaBatch;

use crate::continuous_batch_scheduler::contributions::Contributions;

pub struct BatchPass<'batch> {
    pub batch: &'batch mut LlamaBatch<'static>,
    pub contributions: Contributions,
}

impl<'batch> BatchPass<'batch> {
    pub fn new(batch: &'batch mut LlamaBatch<'static>) -> Self {
        batch.clear();

        Self {
            batch,
            contributions: Contributions::default(),
        }
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.contributions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings::SampledToken;
    use llama_cpp_bindings::llama_batch::LlamaBatch;
    use llama_cpp_bindings::token::LlamaToken;

    use super::BatchPass;

    #[test]
    fn new_clears_tokens_left_by_the_previous_pass() {
        let mut batch = LlamaBatch::new(16, 1).unwrap();

        batch
            .add(&SampledToken::Content(LlamaToken::new(1)), 0, &[0], true)
            .unwrap();

        let batch_pass = BatchPass::new(&mut batch);

        assert_eq!(batch_pass.batch.n_tokens(), 0);
        assert!(batch_pass.is_empty());
    }
}
