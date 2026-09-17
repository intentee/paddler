use crate::continuous_batch_active_request::ContinuousBatchActiveRequest;

pub fn run(requests: &mut [ContinuousBatchActiveRequest]) {
    for request in requests {
        request.token_classifier.discard_pending_prompt_tokens();
    }
}
