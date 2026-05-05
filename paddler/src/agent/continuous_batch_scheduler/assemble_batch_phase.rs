use anyhow::Result;
use llama_cpp_bindings::SampledToken;

use crate::agent::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::agent::continuous_batch_request_phase::ContinuousBatchRequestPhase;
use crate::agent::continuous_batch_scheduler::batch_pass::BatchPass;
use crate::agent::continuous_batch_scheduler::generating_contribution::GeneratingContribution;
use crate::agent::continuous_batch_scheduler::ingesting_contribution::IngestingContribution;

pub struct AssembleBatchPhase {
    pub batch_n_tokens: usize,
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

            if tokens_added >= self.batch_n_tokens {
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
            let available_space = self
                .batch_n_tokens
                .saturating_sub(pass.contributions.current_batch_token_count);
            let chunk_size = remaining.len().min(available_space);

            if chunk_size == 0 {
                continue;
            }

            let chunk = &request.prompt_tokens[request.prompt_tokens_ingested
                ..request.prompt_tokens_ingested + chunk_size];
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
