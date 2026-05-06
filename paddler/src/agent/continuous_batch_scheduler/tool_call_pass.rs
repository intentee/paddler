use llama_cpp_bindings::SampledToken;
use paddler_types::generated_token_result::GeneratedTokenResult;

use crate::agent::continuous_batch_scheduler::classified_token::ClassifiedToken;
use crate::tool_call_event::ToolCallEvent;
use crate::tool_call_pipeline::ToolCallPipeline;

pub struct ToolCallPass;

impl ToolCallPass {
    #[must_use]
    pub fn run(
        self,
        pipeline: Option<&mut ToolCallPipeline>,
        classified: &ClassifiedToken,
        _piece: &str,
    ) -> Option<GeneratedTokenResult> {
        let pipeline = pipeline?;

        if matches!(classified.sampled_token, SampledToken::ToolCall(_)) {
            pipeline.feed(&classified.raw_piece);
        }

        if !classified.was_in_tool_call || classified.is_in_tool_call {
            return None;
        }

        finalize_pipeline_to_event(pipeline)
    }
}

#[must_use]
pub fn finalize_pipeline_to_event(pipeline: &mut ToolCallPipeline) -> Option<GeneratedTokenResult> {
    match pipeline.finalize() {
        ToolCallEvent::Resolved(parsed) => Some(GeneratedTokenResult::ToolCallParsed(parsed)),
        ToolCallEvent::ParseFailed(err) => {
            Some(GeneratedTokenResult::ToolCallParseFailed(err.to_string()))
        }
        ToolCallEvent::ValidationFailed(errors) => Some(GeneratedTokenResult::ToolCallValidationFailed(
            errors.into_iter().map(|err| err.to_string()).collect(),
        )),
        ToolCallEvent::Pending => None,
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings::SampledToken;
    use llama_cpp_bindings::token::LlamaToken;

    use super::ToolCallPass;
    use crate::agent::continuous_batch_scheduler::classified_token::ClassifiedToken;

    fn classified(was: bool, is: bool, sampled: SampledToken) -> ClassifiedToken {
        ClassifiedToken {
            sampled_token: sampled,
            was_in_tool_call: was,
            is_in_tool_call: is,
            visible_piece: String::new(),
            raw_piece: String::new(),
        }
    }

    #[test]
    fn pipeline_none_returns_none_for_content_token() {
        let result = ToolCallPass.run(
            None,
            &classified(false, false, SampledToken::Content(LlamaToken::new(1))),
            "hello",
        );

        assert!(result.is_none());
    }

    #[test]
    fn pipeline_none_returns_none_for_tool_call_token() {
        let result = ToolCallPass.run(
            None,
            &classified(true, true, SampledToken::ToolCall(LlamaToken::new(2))),
            "{",
        );

        assert!(result.is_none());
    }

    #[test]
    fn pipeline_none_returns_none_on_transition_out() {
        let result = ToolCallPass.run(
            None,
            &classified(true, false, SampledToken::ToolCall(LlamaToken::new(3))),
            "}",
        );

        assert!(result.is_none());
    }
}
