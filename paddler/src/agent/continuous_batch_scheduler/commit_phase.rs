use crate::agent::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::agent::continuous_batch_request_phase::ContinuousBatchRequestPhase;
use crate::agent::continuous_batch_scheduler::batch_pass::BatchPass;

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    reason = "chunk sizes fit in i32 for llama.cpp position arithmetic"
)]
pub fn run(pass: BatchPass, requests: &mut [ContinuousBatchActiveRequest]) {
    for contribution in pass.contributions.generating {
        let request = &mut requests[contribution.request_index];

        request.pending_sampled_token = None;
        request.i_batch = Some(contribution.batch_position);
        request.current_token_position += 1;
    }

    for contribution in pass.contributions.ingesting {
        let request = &mut requests[contribution.request_index];

        request.prompt_tokens_ingested += contribution.chunk_size;
        request.current_token_position += contribution.chunk_size as i32;

        if contribution.is_last_chunk {
            request.i_batch = Some(contribution.last_batch_position);
            request.phase = ContinuousBatchRequestPhase::Generating;
        }
    }
}
