use anyhow::Result;

use crate::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::continuous_batch_scheduler::batch_pass::BatchPass;

pub fn run(pass: BatchPass, requests: &mut [ContinuousBatchActiveRequest]) -> Result<()> {
    for contribution in pass.contributions.generating {
        requests[contribution.request_index]
            .state
            .apply_generating_contribution(contribution.batch_position);
    }

    for contribution in pass.contributions.ingesting {
        let request = &mut requests[contribution.request_index];
        let committed_prompt_tokens = request.token_classifier.commit_prompt_tokens();

        request.state.apply_ingesting_contribution(
            committed_prompt_tokens,
            contribution.is_last_chunk,
            contribution.last_batch_position,
        )?;
    }

    Ok(())
}
