use std::mem::take;

use llama_cpp_bindings::ChatMessageParseOutcome;
use llama_cpp_bindings::ChatMessageParser;
use llama_cpp_bindings::ParsedToolCall;
use llama_cpp_bindings::RawChatMessage;
use llama_cpp_bindings::token_piece::TokenPiece;

use paddler_messaging::generated_tool_call::GeneratedToolCall;
use paddler_messaging::raw_tool_call_tokens::RawToolCallTokens;
use paddler_tool_call_validator::tool_call_validator::ToolCallValidator;

use crate::tool_call_buffer::ToolCallBuffer;
use crate::tool_call_event::ToolCallEvent;

pub struct ToolCallPipeline {
    buffer: ToolCallBuffer,
    chat_message_parser: ChatMessageParser,
    tool_calls_generated: usize,
    validator: ToolCallValidator,
}

impl ToolCallPipeline {
    #[must_use]
    pub fn new(chat_message_parser: ChatMessageParser, validator: ToolCallValidator) -> Self {
        Self {
            buffer: ToolCallBuffer::default(),
            chat_message_parser,
            tool_calls_generated: 0,
            validator,
        }
    }

    pub fn feed(&mut self, piece: &TokenPiece) {
        self.buffer.append(piece);
    }

    #[must_use]
    pub const fn buffer_is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn finalize(&mut self) -> ToolCallEvent {
        let ToolCallBuffer {
            accumulated: input,
            holds_visible_text,
        } = take(&mut self.buffer);

        if input.is_empty() {
            return ToolCallEvent::Resolved(Vec::new());
        }

        match self.chat_message_parser.parse(&input, false) {
            Ok(ChatMessageParseOutcome::Recognized(parsed))
                if parsed.tool_calls.is_empty() && holds_visible_text =>
            {
                ToolCallEvent::UnrecognizedFormat(RawToolCallTokens {
                    text: input,
                    ffi_error_message: String::new(),
                })
            }
            Ok(ChatMessageParseOutcome::Recognized(parsed)) => {
                self.validate_resolved(parsed.tool_calls)
            }
            Ok(ChatMessageParseOutcome::Unrecognized(RawChatMessage {
                text,
                ffi_error_message,
                ..
            })) => ToolCallEvent::UnrecognizedFormat(RawToolCallTokens {
                text,
                ffi_error_message,
            }),
            Err(parse_error) => ToolCallEvent::ParseFailed(parse_error),
        }
    }

    fn validate_resolved(&mut self, tool_calls: Vec<ParsedToolCall>) -> ToolCallEvent {
        let validation_errors: Vec<_> = tool_calls
            .iter()
            .filter_map(|tool_call| self.validator.validate(tool_call).err())
            .collect();

        if validation_errors.is_empty() {
            ToolCallEvent::Resolved(self.number_across_the_generation(tool_calls))
        } else {
            ToolCallEvent::ValidationFailed(validation_errors)
        }
    }

    fn number_across_the_generation(
        &mut self,
        tool_calls: Vec<ParsedToolCall>,
    ) -> Vec<GeneratedToolCall> {
        tool_calls
            .into_iter()
            .map(|tool_call| {
                let generated_tool_call =
                    GeneratedToolCall::at_index(tool_call, self.tool_calls_generated);

                self.tool_calls_generated += 1;

                generated_tool_call
            })
            .collect()
    }
}
