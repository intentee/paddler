use llama_cpp_bindings::SampledToken;
use paddler_types::generated_token_result::GeneratedTokenResult;

use crate::agent::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::agent::continuous_batch_scheduler::classified_token::ClassifiedToken;
use crate::agent::continuous_batch_scheduler::emit_token_outcome::EmitTokenOutcome;

pub struct EmitTokenPhase;

impl EmitTokenPhase {
    pub fn run(
        &self,
        request: &mut ContinuousBatchActiveRequest,
        classified: &ClassifiedToken,
    ) -> EmitTokenOutcome {
        if classified.visible_piece.is_empty() {
            return EmitTokenOutcome::Emitted(String::new());
        }

        let piece = classified.visible_piece.clone();
        let event = match classified.sampled_token {
            SampledToken::Content(_) => GeneratedTokenResult::ContentToken(piece.clone()),
            SampledToken::Reasoning(_) => GeneratedTokenResult::ReasoningToken(piece.clone()),
            SampledToken::ToolCall(_) => GeneratedTokenResult::ToolCallToken(piece.clone()),
            SampledToken::Undeterminable(_) => {
                GeneratedTokenResult::UndeterminableToken(piece.clone())
            }
        };

        if request.generated_tokens_tx.send(event).is_err() {
            return EmitTokenOutcome::ChannelDropped;
        }

        EmitTokenOutcome::Emitted(piece)
    }
}
