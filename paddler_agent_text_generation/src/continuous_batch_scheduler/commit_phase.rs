use crate::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::continuous_batch_scheduler::batch_pass::BatchPass;
use crate::continuous_batch_scheduler::contributions::Contributions;

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
                    ..
                },
            ..
        }: BatchPass,
    ) {
        for contribution in generating {
            self.requests[contribution.request_index]
                .state
                .apply_generating_contribution(contribution.batch_position);
        }

        for contribution in ingesting {
            self.requests[contribution.request_index]
                .state
                .apply_ingesting_contribution(&contribution);
        }
    }
}
