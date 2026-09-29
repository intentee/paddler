use llama_cpp_bindings::SampledToken;
use paddler_messaging::generated_token_result::GeneratedTokenResult;

use crate::continuous_batch_scheduler::classified_token::ClassifiedToken;
use crate::tool_call_pipeline::ToolCallPipeline;

pub enum ToolCallHandling {
    Parsed(ToolCallPipeline),
    Streamed,
}

impl ToolCallHandling {
    pub fn feed(&mut self, classified: &ClassifiedToken) {
        if let Self::Parsed(pipeline) = self
            && matches!(classified.sampled_token, SampledToken::ToolCall(_))
        {
            pipeline.feed(classified.piece.raw());
        }
    }

    pub fn resolve_on_section_exit(
        &mut self,
        classified: &ClassifiedToken,
    ) -> Option<GeneratedTokenResult> {
        match self {
            Self::Parsed(pipeline)
                if classified.was_in_tool_call && !classified.is_in_tool_call =>
            {
                Some(pipeline.finalize().into_generated_token_result())
            }
            Self::Parsed(_) | Self::Streamed => None,
        }
    }

    pub fn resolve_at_completion(&mut self) -> Option<GeneratedTokenResult> {
        match self {
            Self::Parsed(pipeline) if !pipeline.buffer_is_empty() => {
                Some(pipeline.finalize().into_generated_token_result())
            }
            Self::Parsed(_) | Self::Streamed => None,
        }
    }
}
