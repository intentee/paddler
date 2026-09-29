use paddler_messaging::generated_token_result::GeneratedTokenResult;

use crate::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::continuous_batch_scheduler::batch_pass::BatchPass;

pub fn run(pass: BatchPass, requests: &mut [ContinuousBatchActiveRequest], description: &str) {
    let generating_request_indices = pass
        .contributions
        .generating
        .iter()
        .map(|contribution| contribution.request_index);
    let ingesting_request_indices = pass
        .contributions
        .ingesting
        .iter()
        .map(|contribution| contribution.request_index);

    for request_index in generating_request_indices.chain(ingesting_request_indices) {
        requests[request_index]
            .complete_with_outcome(GeneratedTokenResult::DecodeFailed(description.to_owned()));
    }
}
