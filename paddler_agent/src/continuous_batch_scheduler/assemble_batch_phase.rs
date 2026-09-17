use anyhow::Context as _;
use anyhow::Result;
use llama_cpp_bindings::SampledToken;

use crate::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::continuous_batch_generating_state::ContinuousBatchGeneratingState;
use crate::continuous_batch_request_phase::ContinuousBatchRequestPhase;
use crate::continuous_batch_scheduler::batch_pass::BatchPass;
use crate::continuous_batch_scheduler::generating_contribution::GeneratingContribution;
use crate::continuous_batch_scheduler::generating_slot::GeneratingSlot;
use crate::continuous_batch_scheduler::ingesting_contribution::IngestingContribution;

fn compute_ingesting_chunk_size(
    remaining_prompt_len: usize,
    n_batch: usize,
    current_batch_token_count: usize,
) -> usize {
    let available_space = n_batch.saturating_sub(current_batch_token_count);

    remaining_prompt_len.min(available_space)
}

/// # Errors
/// Forwards [`LlamaBatch::add`] failures verbatim.
fn fill_generating_slots(
    pass: &mut BatchPass,
    n_batch: usize,
    slots: impl Iterator<Item = GeneratingSlot>,
) -> Result<usize> {
    let mut tokens_added: usize = 0;

    for GeneratingSlot {
        request_index,
        sampled_token,
        position,
        sequence_id,
    } in slots
    {
        if tokens_added >= n_batch {
            break;
        }

        let batch_position = pass.batch.n_tokens();

        pass.batch
            .add(&sampled_token, position, &[sequence_id], true)?;

        pass.contributions.generating.push(GeneratingContribution {
            request_index,
            batch_position,
        });

        tokens_added += 1;
    }

    Ok(tokens_added)
}

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
        fill_generating_slots(
            pass,
            self.n_batch,
            requests
                .iter()
                .enumerate()
                .filter_map(|(request_index, request)| {
                    let ContinuousBatchRequestPhase::Generating(
                        ContinuousBatchGeneratingState::AwaitingBatchSlot { sampled_token },
                    ) = &request.state.phase
                    else {
                        return None;
                    };

                    Some(GeneratingSlot {
                        request_index,
                        sampled_token: *sampled_token,
                        position: request.state.current_token_position,
                        sequence_id: request.sequence_id_guard.sequence_id(),
                    })
                }),
        )
    }

    fn fill_ingesting(
        &self,
        pass: &mut BatchPass,
        requests: &mut [ContinuousBatchActiveRequest],
    ) -> Result<()> {
        for (request_index, request) in requests.iter_mut().enumerate() {
            let ContinuousBatchRequestPhase::Ingesting(ingesting_state) = &mut request.state.phase
            else {
                continue;
            };

            let chunk_size = compute_ingesting_chunk_size(
                ingesting_state.remaining_prompt_tokens().len(),
                self.n_batch,
                pass.contributions.current_batch_token_count,
            );

            if chunk_size == 0 {
                continue;
            }

            let chunk_start = ingesting_state.prompt_tokens_ingested;
            let is_last_chunk = chunk_start + chunk_size >= ingesting_state.prompt_tokens.len();
            let sequence_id = request.sequence_id_guard.sequence_id();
            let current_token_position = request.state.current_token_position;

            for offset in 0..chunk_size {
                let absolute_token_index = chunk_start + offset;
                let token = ingesting_state.prompt_tokens[absolute_token_index];
                let position = current_token_position
                    + i32::try_from(offset).context("token offset does not fit in i32")?;
                let is_last_token_of_prompt = is_last_chunk && offset == chunk_size - 1;

                if ingesting_state.is_already_staged_into_classifier(absolute_token_index) {
                    pass.batch.add(
                        &SampledToken::Content(token),
                        position,
                        &[sequence_id],
                        is_last_token_of_prompt,
                    )?;
                } else {
                    request.token_classifier.feed_prompt_to_batch(
                        &mut pass.batch,
                        token,
                        position,
                        &[sequence_id],
                        is_last_token_of_prompt,
                    )?;
                    ingesting_state.prompt_tokens_staged_into_classifier = absolute_token_index + 1;
                }
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

#[cfg(test)]
mod tests {
    use llama_cpp_bindings::SampledToken;
    use llama_cpp_bindings::token::LlamaToken;

    use super::AssembleBatchPhase;
    use super::GeneratingSlot;
    use super::compute_ingesting_chunk_size;
    use super::fill_generating_slots;
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

    fn slot(request_index: usize) -> GeneratingSlot {
        GeneratingSlot {
            request_index,
            sampled_token: SampledToken::Content(LlamaToken::new(1)),
            position: 0,
            sequence_id: 0,
        }
    }

    #[test]
    fn every_waiting_slot_contributes_one_token_to_the_batch() {
        let mut pass = BatchPass::new(16, 1).unwrap();

        let added = fill_generating_slots(&mut pass, 16, [slot(0), slot(1)].into_iter()).unwrap();

        assert_eq!(added, 2);
        assert_eq!(pass.batch.n_tokens(), 2);
        assert_eq!(pass.contributions.generating.len(), 2);
        assert_eq!(pass.contributions.generating[1].batch_position, 1);
    }

    #[test]
    fn slots_beyond_the_batch_token_budget_are_left_for_the_next_pass() {
        let mut pass = BatchPass::new(16, 1).unwrap();

        let added =
            fill_generating_slots(&mut pass, 2, [slot(0), slot(1), slot(2)].into_iter()).unwrap();

        assert_eq!(added, 2, "the third slot must wait for the next batch");
        assert_eq!(pass.contributions.generating.len(), 2);
    }

    #[test]
    fn a_batch_that_cannot_hold_another_token_forwards_the_add_failure() {
        let mut pass = BatchPass::new(1, 1).unwrap();

        assert!(fill_generating_slots(&mut pass, 16, [slot(0), slot(1)].into_iter()).is_err());
    }
}
