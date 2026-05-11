use anyhow::Result;
use llama_cpp_bindings::SampledToken;

use crate::agent::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::agent::continuous_batch_request_phase::ContinuousBatchRequestPhase;
use crate::agent::continuous_batch_scheduler::batch_pass::BatchPass;
use crate::agent::continuous_batch_scheduler::generating_contribution::GeneratingContribution;
use crate::agent::continuous_batch_scheduler::ingesting_contribution::IngestingContribution;

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
            if !matches!(request.phase, ContinuousBatchRequestPhase::Generating) {
                continue;
            }

            let Some(pending_token) = request.pending_sampled_token else {
                continue;
            };

            if tokens_added >= self.n_batch {
                break;
            }

            let batch_position = pass.batch.n_tokens();

            pass.batch.add(
                &pending_token,
                request.current_token_position,
                &[request.sequence_id],
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

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        reason = "token counts and positions fit in i32 for llama.cpp FFI"
    )]
    fn fill_ingesting(
        &self,
        pass: &mut BatchPass,
        requests: &[ContinuousBatchActiveRequest],
    ) -> Result<()> {
        for (request_index, request) in requests.iter().enumerate() {
            if !matches!(request.phase, ContinuousBatchRequestPhase::Ingesting) {
                continue;
            }

            let remaining = request.remaining_prompt_tokens();
            let chunk_size = compute_ingesting_chunk_size(
                remaining.len(),
                self.n_batch,
                pass.contributions.current_batch_token_count,
            );

            if chunk_size == 0 {
                continue;
            }

            let chunk = &request.prompt_tokens
                [request.prompt_tokens_ingested..request.prompt_tokens_ingested + chunk_size];
            let is_last_chunk =
                request.prompt_tokens_ingested + chunk_size >= request.prompt_tokens.len();

            for (offset, token) in chunk.iter().enumerate() {
                let position = request.current_token_position + offset as i32;
                let is_last_token_of_prompt = is_last_chunk && offset == chunk_size - 1;

                pass.batch.add(
                    &SampledToken::Content(*token),
                    position,
                    &[request.sequence_id],
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
    use super::compute_ingesting_chunk_size;

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
