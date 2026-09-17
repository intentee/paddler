use crate::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::continuous_batch_scheduler::classified_token::ClassifiedToken;
use crate::continuous_batch_scheduler::client_stream_status::ClientStreamStatus;
use crate::continuous_batch_scheduler::emit_token_outcome::EmitTokenOutcome;
use crate::continuous_batch_scheduler::emit_token_phase;
use crate::continuous_batch_scheduler::tool_call_pass;

pub fn run(
    request: &mut ContinuousBatchActiveRequest,
    classified_tokens: &[ClassifiedToken],
) -> ClientStreamStatus {
    for classified in classified_tokens {
        match emit_token_phase::run(request, classified) {
            EmitTokenOutcome::Emitted(_) => {}
            EmitTokenOutcome::ChannelDropped => return ClientStreamStatus::Dropped,
        }

        if let Some(event) = tool_call_pass::run(request.tool_call_pipeline.as_mut(), classified)
            && request.generated_tokens_tx.send(event).is_err()
        {
            return ClientStreamStatus::Dropped;
        }
    }

    ClientStreamStatus::Open
}
