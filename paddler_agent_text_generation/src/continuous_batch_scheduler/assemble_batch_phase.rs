use llama_cpp_bindings::batch_add_error::BatchAddError;
use llama_cpp_bindings::sampled_token::SampledToken;

use crate::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::continuous_batch_generation_step::ContinuousBatchGenerationStep;
use crate::continuous_batch_request_phase::ContinuousBatchRequestPhase;
use crate::continuous_batch_scheduler::batch_pass::BatchPass;
use crate::continuous_batch_scheduler::generating_contribution::GeneratingContribution;
use crate::continuous_batch_scheduler::ingesting_contribution::IngestingContribution;

fn compute_ingesting_chunk_size(
    remaining_prompt_len: usize,
    n_batch: usize,
    current_batch_token_count: usize,
) -> usize {
    let available_space = n_batch.saturating_sub(current_batch_token_count);
    remaining_prompt_len.min(available_space)
}

pub struct AssembleBatchPhase;

impl AssembleBatchPhase {
    pub fn run(
        pass: &mut BatchPass,
        requests: &mut [ContinuousBatchActiveRequest<'_>],
    ) -> Result<(), BatchAddError> {
        Self::fill_generating(pass, requests)?;
        Self::fill_ingesting(pass, requests)?;

        Ok(())
    }

    fn fill_generating(
        pass: &mut BatchPass,
        requests: &[ContinuousBatchActiveRequest<'_>],
    ) -> Result<(), BatchAddError> {
        for (request_index, request) in requests.iter().enumerate() {
            let ContinuousBatchRequestPhase::Generating(
                ContinuousBatchGenerationStep::AwaitingDecode(pending_token),
            ) = &request.state.phase
            else {
                continue;
            };

            let batch_position = pass.batch.n_tokens();

            pass.batch.add(
                pending_token,
                request.state.current_token_position,
                &[request.sequence_id_guard.sequence_id().llama_sequence_id()],
                true,
            )?;

            pass.contributions.generating.push(GeneratingContribution {
                request_index,
                batch_position,
            });
        }

        Ok(())
    }

    fn fill_ingesting(
        pass: &mut BatchPass,
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
            let chunk_size = compute_ingesting_chunk_size(
                remaining.len(),
                pass.batch.limits().n_batch as usize,
                pass.batch.n_tokens() as usize,
            );

            if chunk_size == 0 {
                continue;
            }

            let chunk = &request.state.prompt_tokens[request.state.prompt_tokens_ingested
                ..request.state.prompt_tokens_ingested + chunk_size];
            let is_last_chunk = request.state.prompt_tokens_ingested + chunk_size
                >= request.state.prompt_tokens.len();
            let mut next_token_position = request.state.current_token_position;

            for (offset, (position, token)) in (request.state.current_token_position..)
                .zip(chunk)
                .enumerate()
            {
                pass.batch.add(
                    &SampledToken::Content(*token),
                    position,
                    &[request.sequence_id_guard.sequence_id().llama_sequence_id()],
                    is_last_chunk && offset == chunk_size - 1,
                )?;

                next_token_position = position + 1;
            }

            pass.contributions.ingesting.push(IngestingContribution {
                request_index,
                chunk_size,
                is_last_chunk,
                last_batch_position: pass.batch.n_tokens() - 1,
                next_token_position,
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings::context::context_limits::ContextLimits;
    use llama_cpp_bindings::llama_batch::LlamaBatch;

    use super::AssembleBatchPhase;
    use super::compute_ingesting_chunk_size;
    use crate::continuous_batch_active_request::ContinuousBatchActiveRequest;
    use crate::continuous_batch_scheduler::batch_pass::BatchPass;

    #[test]
    fn run_over_empty_requests_leaves_batch_untouched() {
        let mut batch = LlamaBatch::new(ContextLimits {
            n_batch: 16,
            n_ubatch: 16,
            n_seq_max: 1,
            n_vocab: 2,
        });
        let mut pass = BatchPass::new(&mut batch);
        let mut requests: [ContinuousBatchActiveRequest; 0] = [];

        AssembleBatchPhase::run(&mut pass, &mut requests).unwrap();

        assert_eq!(pass.batch.n_tokens(), 0);
        assert!(pass.is_empty());
    }

    #[test]
    fn chunk_size_is_min_of_remaining_and_available_space() {
        assert_eq!(compute_ingesting_chunk_size(10, 32, 0), 10);
        assert_eq!(compute_ingesting_chunk_size(100, 32, 0), 32);
    }

    #[test]
    fn chunk_size_subtracts_already_used_space_from_batch_capacity() {
        assert_eq!(compute_ingesting_chunk_size(20, 32, 12), 20);
        assert_eq!(compute_ingesting_chunk_size(50, 32, 12), 20);
    }

    #[test]
    fn chunk_size_is_zero_when_batch_already_full() {
        assert_eq!(compute_ingesting_chunk_size(50, 32, 32), 0);
    }

    #[test]
    fn chunk_size_is_zero_when_already_overfilled_via_saturating_sub() {
        assert_eq!(compute_ingesting_chunk_size(50, 32, 40), 0);
    }

    #[test]
    fn chunk_size_is_zero_when_remaining_prompt_is_empty() {
        assert_eq!(compute_ingesting_chunk_size(0, 32, 0), 0);
    }
}
