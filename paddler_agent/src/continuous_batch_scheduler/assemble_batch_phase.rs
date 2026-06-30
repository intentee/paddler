use anyhow::Result;
use llama_cpp_bindings::SampledToken;

use crate::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::continuous_batch_request_phase::ContinuousBatchRequestPhase;
use crate::continuous_batch_scheduler::batch_pass::BatchPass;
use crate::continuous_batch_scheduler::generating_contribution::GeneratingContribution;
use crate::continuous_batch_scheduler::ingesting_contribution::IngestingContribution;

pub struct AssembleBatchPhase {
    pub n_batch: usize,
}

impl AssembleBatchPhase {
    /// # Errors
    /// Forwards `LlamaBatch::add` failures verbatim.
    pub fn run(
        &self,
        pass: &mut BatchPass,
        requests: &mut [ContinuousBatchActiveRequest],
    ) -> Result<()> {
        let added = self.fill_generating(pass, requests)?;
        pass.contributions.current_batch_token_count += added;
        self.fill_ingesting(pass, requests)?;
        Ok(())
    }

    fn fill_generating(
        &self,
        pass: &mut BatchPass,
        requests: &[ContinuousBatchActiveRequest],
    ) -> Result<usize> {
        let mut tokens_added: usize = 0;

        for (request_index, request) in requests.iter().enumerate() {
            if !matches!(request.state.phase, ContinuousBatchRequestPhase::Generating) {
                continue;
            }

            let Some(pending_token) = request.state.pending_sampled_token else {
                continue;
            };

            if tokens_added >= self.n_batch {
                break;
            }

            let batch_position = pass.batch.n_tokens();

            pass.batch.add(
                &pending_token,
                request.state.current_token_position,
                &[i32::from(request.state.sequence_id)],
                true,
            )?;

            pass.contributions.generating.push(GeneratingContribution {
                request_index,
                batch_position,
            });

            tokens_added += 1;
        }

        Ok(tokens_added)
    }

    fn fill_ingesting(
        &self,
        pass: &mut BatchPass,
        requests: &[ContinuousBatchActiveRequest],
    ) -> Result<()> {
        for (request_index, request) in requests.iter().enumerate() {
            if !matches!(request.state.phase, ContinuousBatchRequestPhase::Ingesting) {
                continue;
            }

            let remaining = request.state.remaining_prompt_tokens();
            let chunk_size = compute_ingesting_chunk_size(
                remaining.len(),
                self.n_batch,
                pass.contributions.current_batch_token_count,
            );

            if chunk_size == 0 {
                continue;
            }

            let chunk = &request.state.prompt_tokens[request.state.prompt_tokens_ingested
                ..request.state.prompt_tokens_ingested + chunk_size];
            let is_last_chunk = request.state.prompt_tokens_ingested + chunk_size
                >= request.state.prompt_tokens.len();

            for (position, (offset, token)) in
                (request.state.current_token_position..).zip(chunk.iter().enumerate())
            {
                let is_last_token_of_prompt = is_last_chunk && offset == chunk_size - 1;

                pass.batch.add(
                    &SampledToken::Content(*token),
                    position,
                    &[i32::from(request.state.sequence_id)],
                    is_last_token_of_prompt,
                )?;
            }

            pass.contributions.ingesting.push(IngestingContribution {
                request_index,
                chunk_size,
                is_last_chunk,
                last_batch_position: pass.batch.n_tokens() - 1,
            });

            pass.contributions.current_batch_token_count += chunk_size;
        }

        Ok(())
    }
}

fn compute_ingesting_chunk_size(
    remaining_prompt_len: usize,
    n_batch: usize,
    current_batch_token_count: usize,
) -> usize {
    let available_space = n_batch.saturating_sub(current_batch_token_count);
    remaining_prompt_len.min(available_space)
}

#[cfg(test)]
mod tests {
    use super::AssembleBatchPhase;
    use super::compute_ingesting_chunk_size;
    use crate::continuous_batch_active_request::ContinuousBatchActiveRequest;
    use crate::continuous_batch_scheduler::batch_pass::BatchPass;

    #[test]
    fn run_over_empty_requests_leaves_batch_untouched() {
        let assemble_phase = AssembleBatchPhase { n_batch: 16 };
        let mut pass = BatchPass::new(16, 1).unwrap();
        let mut requests: [ContinuousBatchActiveRequest; 0] = [];

        assemble_phase.run(&mut pass, &mut requests).unwrap();

        assert_eq!(pass.contributions.current_batch_token_count, 0);
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
