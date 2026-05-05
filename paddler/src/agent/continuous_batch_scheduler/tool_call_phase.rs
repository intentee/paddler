use llama_cpp_bindings::SampledToken;
use paddler_types::generated_token_result::GeneratedTokenResult;

use crate::agent::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::agent::continuous_batch_scheduler::classified_token::ClassifiedToken;
use crate::tool_call_event::ToolCallEvent;

pub struct ToolCallPhase;

impl ToolCallPhase {
    pub fn run(
        self,
        request: &mut ContinuousBatchActiveRequest,
        classified: &ClassifiedToken,
        piece: &str,
    ) -> Option<GeneratedTokenResult> {
        if matches!(classified.sampled_token, SampledToken::ToolCall(_))
            && let Some(pipeline) = request.tool_call_pipeline.as_mut()
        {
            pipeline.feed(piece);
        }

        if !classified.was_in_tool_call || classified.is_in_tool_call {
            return None;
        }

        let pipeline = request.tool_call_pipeline.as_mut()?;

        match pipeline.finalize() {
            ToolCallEvent::Resolved(parsed) => Some(GeneratedTokenResult::ToolCallParsed(parsed)),
            ToolCallEvent::ParseFailed(err) => {
                Some(GeneratedTokenResult::ToolCallParseFailed(err.to_string()))
            }
            ToolCallEvent::ValidationFailed(errors) => {
                Some(GeneratedTokenResult::ToolCallValidationFailed(
                    errors.into_iter().map(|err| err.to_string()).collect(),
                ))
            }
            ToolCallEvent::Pending => None,
        }
    }
}
