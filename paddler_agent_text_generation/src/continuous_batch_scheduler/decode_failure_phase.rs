use std::collections::BTreeSet;

use paddler_messaging::generated_token_result::GeneratedTokenResult;

use crate::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::continuous_batch_scheduler::batch_pass::BatchPass;
use crate::continuous_batch_scheduler::contributions::Contributions;
use crate::generating_contribution::GeneratingContribution;
use crate::ingesting_contribution::IngestingContribution;

pub struct DecodeFailurePhase<'requests, 'model> {
    pub requests: &'requests mut [ContinuousBatchActiveRequest<'model>],
}

impl DecodeFailurePhase<'_, '_> {
    pub fn run(
        self,
        BatchPass {
            contributions:
                Contributions {
                    generating,
                    ingesting,
                },
            ..
        }: BatchPass,
        description: &str,
    ) {
        let failed_request_indices: BTreeSet<usize> = generating
            .iter()
            .map(|&GeneratingContribution { request_index, .. }| request_index)
            .chain(
                ingesting
                    .iter()
                    .map(|&IngestingContribution { request_index, .. }| request_index),
            )
            .collect();

        self.requests
            .iter_mut()
            .enumerate()
            .filter(|(request_index, _request)| failed_request_indices.contains(request_index))
            .for_each(|(_request_index, request)| {
                request.complete_with_outcome(GeneratedTokenResult::DecodeFailed(
                    description.to_owned(),
                ));
            });
    }
}
