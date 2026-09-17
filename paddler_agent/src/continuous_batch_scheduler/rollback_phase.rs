use crate::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::continuous_batch_scheduler::batch_pass::BatchPass;

pub fn run(pass: &BatchPass, requests: &mut [ContinuousBatchActiveRequest]) {
    for contribution in &pass.contributions.ingesting {
        requests[contribution.request_index]
            .token_classifier
            .discard_pending_prompt_tokens();
    }
}
