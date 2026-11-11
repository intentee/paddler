use llama_cpp_bindings::chat_reading::ChatReading;
use llama_cpp_bindings::sampled_token_section::SampledTokenSection;
use llama_cpp_bindings_types::token_usage::TokenUsage;

use paddler_messaging::generated_token_result::GeneratedTokenResult;

use crate::classified_token::ClassifiedToken;
use crate::tool_call_event::ToolCallEvent;
use crate::tool_call_pipeline::ToolCallPipeline;

pub enum ToolCallHandling {
    Parsed(ToolCallPipeline),
    Streamed,
}

impl ToolCallHandling {
    pub fn resolve_at_completion(
        &mut self,
        latest_reading: Option<&ChatReading>,
        final_section: SampledTokenSection,
        usage: &TokenUsage,
    ) -> Option<GeneratedTokenResult> {
        match (self, latest_reading) {
            (Self::Parsed(pipeline), Some(reading)) => pipeline
                .resolve_at_completion(reading, final_section, usage)
                .map(ToolCallEvent::into_generated_token_result),
            (Self::Parsed(_) | Self::Streamed, _) => None,
        }
    }

    pub fn resolve_on_section_exit(
        &mut self,
        &ClassifiedToken {
            is_in_tool_call,
            was_in_tool_call,
            ..
        }: &ClassifiedToken,
        latest_reading: Option<&ChatReading>,
    ) -> Option<GeneratedTokenResult> {
        match (self, latest_reading) {
            (Self::Parsed(pipeline), Some(reading)) if was_in_tool_call && !is_in_tool_call => {
                pipeline
                    .resolve_tool_calls_of(reading)
                    .map(ToolCallEvent::into_generated_token_result)
            }
            (Self::Parsed(_) | Self::Streamed, _) => None,
        }
    }
}
