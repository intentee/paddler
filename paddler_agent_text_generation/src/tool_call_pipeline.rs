use std::ops::RangeFrom;

use llama_cpp_bindings::chat_classification::ChatClassification;
use llama_cpp_bindings::chat_message_parse_outcome::ChatMessageParseOutcome;
use llama_cpp_bindings::chat_reading::ChatReading;
use llama_cpp_bindings::raw_chat_message::RawChatMessage;
use llama_cpp_bindings::sampled_token_section::SampledTokenSection;
use llama_cpp_bindings_types::parsed_tool_call::ParsedToolCall;
use llama_cpp_bindings_types::token_usage::TokenUsage;

use paddler_messaging::generated_tool_call::GeneratedToolCall;
use paddler_messaging::raw_tool_call_tokens::RawToolCallTokens;
use paddler_tool_call_validator::tool_call_validator::ToolCallValidator;

use crate::numbered_tool_call::NumberedToolCall;
use crate::tool_call_event::ToolCallEvent;

pub struct ToolCallPipeline {
    tool_call_numbers: RangeFrom<usize>,
    tool_calls_resolved: usize,
    validator: ToolCallValidator,
}

impl ToolCallPipeline {
    #[must_use]
    pub const fn new(validator: ToolCallValidator) -> Self {
        Self {
            tool_call_numbers: 0..,
            tool_calls_resolved: 0,
            validator,
        }
    }

    pub fn resolve_at_completion<'reading>(
        &mut self,
        reading: &'reading ChatReading,
        final_section: SampledTokenSection,
        &TokenUsage {
            tool_call_tokens, ..
        }: &TokenUsage,
    ) -> Option<ToolCallEvent<'reading>> {
        match reading.classification() {
            ChatClassification::OutputDoesNotMatchFormat(RawChatMessage {
                text,
                expected_format,
                ..
            }) if tool_call_tokens > 0 => {
                Some(ToolCallEvent::UnrecognizedFormat(RawToolCallTokens {
                    text: text.clone(),
                    expected_format: expected_format.clone(),
                }))
            }
            ChatClassification::Spans(_) if final_section == SampledTokenSection::ToolCall => {
                self.resolve_tool_calls_of(reading)
            }
            ChatClassification::OutputDoesNotMatchFormat(_) | ChatClassification::Spans(_) => None,
        }
    }

    pub fn resolve_tool_calls_of<'reading>(
        &mut self,
        reading: &'reading ChatReading,
    ) -> Option<ToolCallEvent<'reading>> {
        match reading.parse() {
            Ok(ChatMessageParseOutcome::Recognized(message)) => {
                Some(self.resolve(&message.tool_calls))
            }
            Ok(ChatMessageParseOutcome::Unrecognized(_)) => None,
            Err(parse_failure) => Some(ToolCallEvent::ParseFailed(parse_failure)),
        }
    }

    fn resolve<'reading>(
        &mut self,
        parsed_tool_calls: &[ParsedToolCall],
    ) -> ToolCallEvent<'reading> {
        let mut tool_call_numbers = self.tool_call_numbers.clone();
        let newly_resolved: Vec<GeneratedToolCall> = parsed_tool_calls
            .iter()
            .skip(self.tool_calls_resolved)
            .cloned()
            .zip(&mut tool_call_numbers)
            .map(|(parsed_tool_call, number)| {
                NumberedToolCall {
                    number,
                    parsed_tool_call,
                }
                .into_generated_tool_call()
            })
            .collect();
        let validation_errors: Vec<_> = newly_resolved
            .iter()
            .filter_map(|tool_call| self.validator.validate(tool_call).err())
            .collect();

        self.tool_calls_resolved = self.tool_calls_resolved.max(parsed_tool_calls.len());

        if validation_errors.is_empty() {
            self.tool_call_numbers = tool_call_numbers;

            ToolCallEvent::Resolved(newly_resolved)
        } else {
            ToolCallEvent::ValidationFailed(validation_errors)
        }
    }
}
