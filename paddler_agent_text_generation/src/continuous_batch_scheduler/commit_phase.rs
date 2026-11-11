use crate::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::continuous_batch_scheduler::batch_pass::BatchPass;
use crate::continuous_batch_scheduler::contributions::Contributions;
use crate::generating_contribution::GeneratingContribution;
use crate::ingesting_contribution::IngestingContribution;

pub struct CommitPhase<'requests, 'model> {
    pub requests: &'requests mut [ContinuousBatchActiveRequest<'model>],
}

impl CommitPhase<'_, '_> {
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
    ) {
        let mut generating = generating.into_iter().peekable();
        let mut ingesting = ingesting.into_iter().peekable();

        for (request_index, request) in self.requests.iter_mut().enumerate() {
            if let Some(generating_contribution) = generating.next_if(
                |&GeneratingContribution {
                     request_index: contributing_request_index,
                     ..
                 }| contributing_request_index == request_index,
            ) {
                request
                    .state
                    .apply_generating_contribution(&generating_contribution);
            }

            if let Some(ingesting_contribution) = ingesting.next_if(
                |&IngestingContribution {
                     request_index: contributing_request_index,
                     ..
                 }| contributing_request_index == request_index,
            ) {
                request
                    .state
                    .apply_ingesting_contribution(&ingesting_contribution);
            }
        }
    }
}
