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

/// # Errors
/// Fails when the request is no longer ingesting, or when the chunk size cannot be counted.
fn validated_chunk_size(phase: &ContinuousBatchRequestPhase, chunk_size: usize) -> Result<u64> {
    if !matches!(phase, ContinuousBatchRequestPhase::Ingesting(_)) {
        return Err(anyhow!(
            "an ingesting contribution was committed for a request that is not ingesting"
        ));
    }

    u64::try_from(chunk_size).context("ingested chunk size does not fit in u64")
}

pub fn run(pass: BatchPass, requests: &mut [ContinuousBatchActiveRequest]) -> Result<()> {
    let mut committed_chunk_sizes: Vec<u64> =
        Vec::with_capacity(pass.contributions.ingesting.len());

    for contribution in &pass.contributions.ingesting {
        committed_chunk_sizes.push(validated_chunk_size(
            &requests[contribution.request_index].state.phase,
            contribution.chunk_size,
        )?);
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
    use llama_cpp_bindings::token::LlamaToken;

    use super::restaged_prompt_token_count;
    use super::validated_chunk_size;
    use crate::continuous_batch_generating_state::ContinuousBatchGeneratingState;
    use crate::continuous_batch_ingesting_state::ContinuousBatchIngestingState;
    use crate::continuous_batch_request_phase::ContinuousBatchRequestPhase;
    use crate::continuous_batch_terminal_outcome::ContinuousBatchTerminalOutcome;

    #[test]
    fn an_ingesting_request_reports_its_chunk_size() {
        let phase = ContinuousBatchRequestPhase::Ingesting(ContinuousBatchIngestingState::new(
            vec![LlamaToken::new(1); 4],
        ));

        assert_eq!(validated_chunk_size(&phase, 4).unwrap(), 4);
    }

    #[test]
    fn a_generating_request_cannot_commit_an_ingesting_chunk() {
        let phase = ContinuousBatchRequestPhase::Generating(
            ContinuousBatchGeneratingState::AwaitingSample { batch_index: 0 },
        );

        assert!(validated_chunk_size(&phase, 4).is_err());
    }

    #[test]
    fn a_completed_request_cannot_commit_an_ingesting_chunk() {
        let phase =
            ContinuousBatchRequestPhase::Completed(ContinuousBatchTerminalOutcome::EmitNothing);

        assert!(validated_chunk_size(&phase, 4).is_err());
    }

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
