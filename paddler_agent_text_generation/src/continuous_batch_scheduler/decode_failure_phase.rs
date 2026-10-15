use paddler_messaging::generated_token_result::GeneratedTokenResult;

use crate::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::continuous_batch_scheduler::batch_pass::BatchPass;
use crate::continuous_batch_scheduler::contributions::Contributions;

pub struct DecodeFailurePhase<'requests> {
    pub requests: &'requests mut [ContinuousBatchActiveRequest],
}

impl DecodeFailurePhase<'_> {
    pub fn run(
        self,
        BatchPass {
            contributions:
                Contributions {
                    generating,
                    ingesting,
                    ..
                },
            ..
        }: BatchPass,
        description: &str,
    ) {
        let generating_request_indices = generating
            .iter()
            .map(|contribution| contribution.request_index);
        let ingesting_request_indices = ingesting
            .iter()
            .map(|contribution| contribution.request_index);

        for request_index in generating_request_indices.chain(ingesting_request_indices) {
            self.requests[request_index]
                .complete_with_outcome(GeneratedTokenResult::DecodeFailed(description.to_owned()));
        }
    }
}
