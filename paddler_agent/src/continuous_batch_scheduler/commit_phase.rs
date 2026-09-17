use anyhow::Context as _;
use anyhow::Result;

use crate::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::continuous_batch_request_phase::ContinuousBatchRequestPhase;
use crate::continuous_batch_scheduler::batch_pass::BatchPass;

pub fn run(pass: BatchPass, requests: &mut [ContinuousBatchActiveRequest]) -> Result<()> {
    let mut committed_chunk_sizes: Vec<u64> =
        Vec::with_capacity(pass.contributions.ingesting.len());

    for contribution in &pass.contributions.ingesting {
        if !matches!(
            requests[contribution.request_index].state.phase,
            ContinuousBatchRequestPhase::Ingesting(_)
        ) {
            return Err(anyhow::anyhow!(
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
        let restaged_prompt_tokens = chunk_size.saturating_sub(newly_staged_prompt_tokens);

        if restaged_prompt_tokens > 0 {
            request
                .token_classifier
                .record_prompt_tokens(restaged_prompt_tokens);
        }

        request.state.apply_ingesting_contribution(
            contribution.chunk_size,
            contribution.is_last_chunk,
            contribution.last_batch_position,
        )?;
    }

    Ok(())
}
