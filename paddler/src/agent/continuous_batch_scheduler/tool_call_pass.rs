use llama_cpp_bindings::SampledToken;
use crate::generated_token_result::GeneratedTokenResult;

use crate::agent::continuous_batch_scheduler::classified_token::ClassifiedToken;
use crate::tool_call_pipeline::ToolCallPipeline;

#[must_use]
pub fn run(
    pipeline: Option<&mut ToolCallPipeline>,
    classified: &ClassifiedToken,
) -> Option<GeneratedTokenResult> {
    let pipeline = pipeline?;

    if matches!(classified.sampled_token, SampledToken::ToolCall(_)) {
        pipeline.feed(&classified.raw_piece);
    }

    if !classified.was_in_tool_call || classified.is_in_tool_call {
        return None;
    }

    pipeline.finalize_to_generated_event()
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings::SampledToken;
    use llama_cpp_bindings::token::LlamaToken;

    use super::run;
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
        let result = run(
            None,
            &classified(false, false, SampledToken::Content(LlamaToken::new(1))),
        );

        assert!(result.is_none());
    }

    #[test]
    fn pipeline_none_returns_none_for_tool_call_token() {
        let result = run(
            None,
            &classified(true, true, SampledToken::ToolCall(LlamaToken::new(2))),
        );

        assert!(result.is_none());
    }

    #[test]
    fn pipeline_none_returns_none_on_transition_out() {
        let result = run(
            None,
            &classified(true, false, SampledToken::ToolCall(LlamaToken::new(3))),
        );

        assert!(result.is_none());
    }
}
