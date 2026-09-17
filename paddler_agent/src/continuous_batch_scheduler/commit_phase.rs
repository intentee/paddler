use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;

use crate::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::continuous_batch_request_phase::ContinuousBatchRequestPhase;
use crate::continuous_batch_scheduler::batch_pass::BatchPass;

/// Prompt tokens re-staged after a rolled back decode are already in the classifier, so
/// `commit_prompt_tokens` does not see them and they still have to be recorded as prompt usage.
const fn restaged_prompt_token_count(chunk_size: u64, newly_staged_prompt_tokens: u64) -> u64 {
    chunk_size.saturating_sub(newly_staged_prompt_tokens)
}

pub fn run(pass: BatchPass, requests: &mut [ContinuousBatchActiveRequest]) -> Result<()> {
    let mut committed_chunk_sizes: Vec<u64> =
        Vec::with_capacity(pass.contributions.ingesting.len());

    for contribution in &pass.contributions.ingesting {
        if !matches!(
            requests[contribution.request_index].state.phase,
            ContinuousBatchRequestPhase::Ingesting(_)
        ) {
            return Err(anyhow!(
                "an ingesting contribution was committed for a request that is not ingesting"
            ));
        }

        committed_chunk_sizes.push(
            u64::try_from(contribution.chunk_size)
                .context("ingested chunk size does not fit in u64")?,
        );
    }

    for contribution in pass.contributions.generating {
        requests[contribution.request_index]
            .state
            .apply_generating_contribution(contribution.batch_position);
    }

    for (contribution, chunk_size) in pass
        .contributions
        .ingesting
        .into_iter()
        .zip(committed_chunk_sizes)
    {
        let request = &mut requests[contribution.request_index];
        let newly_staged_prompt_tokens = request.token_classifier.commit_prompt_tokens();

        request
            .token_classifier
            .record_prompt_tokens(restaged_prompt_token_count(
                chunk_size,
                newly_staged_prompt_tokens,
            ));

        request.state.apply_ingesting_contribution(
            contribution.chunk_size,
            contribution.is_last_chunk,
            contribution.last_batch_position,
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::restaged_prompt_token_count;

    #[test]
    fn a_chunk_staged_entirely_by_this_pass_needs_no_extra_recording() {
        assert_eq!(restaged_prompt_token_count(8, 8), 0);
    }

    #[test]
    fn a_chunk_replayed_after_a_rollback_records_every_token_itself() {
        assert_eq!(restaged_prompt_token_count(8, 0), 8);
    }

    #[test]
    fn a_partially_replayed_chunk_records_only_the_tokens_the_classifier_already_held() {
        assert_eq!(restaged_prompt_token_count(8, 3), 5);
    }

    #[test]
    fn staging_more_than_the_chunk_never_underflows() {
        assert_eq!(restaged_prompt_token_count(3, 8), 0);
    }
}
