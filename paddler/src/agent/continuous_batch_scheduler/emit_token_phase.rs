use llama_cpp_bindings::SampledToken;
use llama_cpp_bindings::model::LlamaModel;
use paddler_types::generated_token_result::GeneratedTokenResult;

use crate::agent::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::agent::continuous_batch_scheduler::classified_token::ClassifiedToken;
use crate::agent::continuous_batch_scheduler::emit_token_outcome::EmitTokenOutcome;

pub struct EmitTokenPhase<'model> {
    pub model: &'model LlamaModel,
}

impl EmitTokenPhase<'_> {
    pub fn run(
        &self,
        request: &mut ContinuousBatchActiveRequest,
        classified: &ClassifiedToken,
    ) -> EmitTokenOutcome {
        let piece = match self.model.token_to_piece(
            &classified.sampled_token,
            &mut request.utf8_decoder,
            true,
            None,
        ) {
            Ok(piece) => piece,
            Err(err) => {
                return EmitTokenOutcome::PieceConversionFailed(err.to_string());
            }
        };

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
