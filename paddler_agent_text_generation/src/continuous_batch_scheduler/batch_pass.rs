use std::iter::once;

use llama_cpp_bindings::batch_add_error::BatchAddError;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::sampled_token::SampledToken;

use crate::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::continuous_batch_generation_step::ContinuousBatchGenerationStep;
use crate::continuous_batch_request_phase::ContinuousBatchRequestPhase;
use crate::continuous_batch_scheduler::contributions::Contributions;
use crate::generating_contribution::GeneratingContribution;
use crate::ingesting_contribution::IngestingContribution;

fn compute_ingesting_chunk_size(
    remaining_prompt_len: usize,
    n_batch: usize,
    generating_count: usize,
    ingesting_token_count: usize,
) -> usize {
    let available_space = n_batch
        .saturating_sub(generating_count)
        .saturating_sub(ingesting_token_count);
    remaining_prompt_len.min(available_space)
}

pub struct BatchPass<'batch> {
    pub batch: &'batch mut LlamaBatch,
    pub contributions: Contributions,
}

impl<'batch> BatchPass<'batch> {
    pub fn new(batch: &'batch mut LlamaBatch) -> Self {
        batch.clear();

        Self {
            batch,
            contributions: Contributions::default(),
        }
    }

    /// # Errors
    ///
    /// Returns the [`BatchAddError`] of the first token the batch refuses.
    pub fn assemble(
        &mut self,
        requests: &[ContinuousBatchActiveRequest<'_>],
    ) -> Result<(), BatchAddError> {
        self.fill_generating(requests)?;
        self.fill_ingesting(requests)?;

        Ok(())
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.contributions.is_empty()
    }

    fn fill_generating(
        &mut self,
        requests: &[ContinuousBatchActiveRequest<'_>],
    ) -> Result<(), BatchAddError> {
        for (request_index, request) in requests.iter().enumerate() {
            let ContinuousBatchRequestPhase::Generating(
                ContinuousBatchGenerationStep::AwaitingDecode(pending_token),
            ) = &request.state.phase
            else {
                continue;
            };

            let batch_position = self.batch.n_tokens();
            let mut token_positions = request.state.current_token_position..;

            for (token, position) in once(pending_token).zip(&mut token_positions) {
                self.batch.add(
                    token,
                    position,
                    &[request.sequence_id_guard.sequence_id().llama_sequence_id()],
                    true,
                )?;
            }

            self.contributions.generating.push(GeneratingContribution {
                request_index,
                batch_position,
                next_token_position: token_positions.start,
            });
        }

        Ok(())
    }

    fn fill_ingesting(
        &mut self,
        requests: &[ContinuousBatchActiveRequest<'_>],
    ) -> Result<(), BatchAddError> {
        for (request_index, request) in requests.iter().enumerate() {
            if !matches!(
                request.state.phase,
                ContinuousBatchRequestPhase::IngestingText
            ) {
                continue;
            }

            let remaining = request.state.remaining_prompt_tokens();
            let remaining_count = remaining.len();
            let ingesting_token_count = self
                .contributions
                .ingesting
                .iter()
                .map(|IngestingContribution { chunk_size, .. }| *chunk_size)
                .sum::<usize>();
            let chunk_size = compute_ingesting_chunk_size(
                remaining_count,
                self.batch.limits().n_batch as usize,
                self.contributions.generating.len(),
                ingesting_token_count,
            );

            if chunk_size == 0 {
                continue;
            }

            let is_last_chunk = chunk_size >= remaining_count;
            let mut token_positions = request.state.current_token_position..;
            let mut chunk = remaining
                .iter()
                .take(chunk_size)
                .zip(&mut token_positions)
                .peekable();
            let mut last_batch_position = self.batch.n_tokens();

            while let Some((token, position)) = chunk.next() {
                last_batch_position = self.batch.n_tokens();

                self.batch.add(
                    &SampledToken::Content(*token),
                    position,
                    &[request.sequence_id_guard.sequence_id().llama_sequence_id()],
                    is_last_chunk && chunk.peek().is_none(),
                )?;
            }

            self.contributions.ingesting.push(IngestingContribution {
                request_index,
                chunk_size,
                is_last_chunk,
                last_batch_position,
                next_token_position: token_positions.start,
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings::context::context_limits::ContextLimits;
    use llama_cpp_bindings::llama_batch::LlamaBatch;
    use llama_cpp_bindings::sampled_token::SampledToken;
    use llama_cpp_bindings::token::LlamaToken;

    use super::BatchPass;
    use super::compute_ingesting_chunk_size;
    use crate::continuous_batch_active_request::ContinuousBatchActiveRequest;

    #[test]
    fn new_clears_tokens_left_by_the_previous_pass() {
        let mut batch = LlamaBatch::new(ContextLimits {
            n_batch: 16,
            n_ubatch: 16,
            n_seq_max: 1,
            n_vocab: 2,
        });

        batch
            .add(&SampledToken::Content(LlamaToken::new(1)), 0, &[0], true)
            .unwrap();

        let batch_pass = BatchPass::new(&mut batch);

        assert_eq!(batch_pass.batch.n_tokens(), 0);
        assert!(batch_pass.is_empty());
    }

    #[test]
    fn assembling_empty_requests_leaves_the_batch_untouched() {
        let mut batch = LlamaBatch::new(ContextLimits {
            n_batch: 16,
            n_ubatch: 16,
            n_seq_max: 1,
            n_vocab: 2,
        });
        let mut pass = BatchPass::new(&mut batch);
        let requests: [ContinuousBatchActiveRequest; 0] = [];

        pass.assemble(&requests).unwrap();

        assert_eq!(pass.batch.n_tokens(), 0);
        assert!(pass.is_empty());
    }

    #[test]
    fn chunk_size_is_min_of_remaining_and_available_space() {
        assert_eq!(compute_ingesting_chunk_size(10, 32, 0, 0), 10);
        assert_eq!(compute_ingesting_chunk_size(100, 32, 0, 0), 32);
    }

    #[test]
    fn chunk_size_subtracts_already_used_space_from_batch_capacity() {
        assert_eq!(compute_ingesting_chunk_size(20, 32, 2, 10), 20);
        assert_eq!(compute_ingesting_chunk_size(50, 32, 2, 10), 20);
    }

    #[test]
    fn chunk_size_is_zero_when_batch_already_full() {
        assert_eq!(compute_ingesting_chunk_size(50, 32, 4, 28), 0);
    }

    #[test]
    fn chunk_size_is_zero_when_already_overfilled_via_saturating_sub() {
        assert_eq!(compute_ingesting_chunk_size(50, 32, 40, 0), 0);
        assert_eq!(compute_ingesting_chunk_size(50, 32, 8, 32), 0);
    }

    #[test]
    fn chunk_size_is_zero_when_remaining_prompt_is_empty() {
        assert_eq!(compute_ingesting_chunk_size(0, 32, 0, 0), 0);
    }
}
