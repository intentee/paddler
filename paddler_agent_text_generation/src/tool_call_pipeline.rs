use llama_cpp_bindings::ChatMessageParseOutcome;
use llama_cpp_bindings::ChatMessageParser;
use llama_cpp_bindings::ParsedToolCall;
use llama_cpp_bindings::RawChatMessage;

use paddler_messaging::raw_tool_call_tokens::RawToolCallTokens;
use paddler_tool_call_validator::tool_call_validator::ToolCallValidator;

use crate::tool_call_buffer::ToolCallBuffer;
use crate::tool_call_event::ToolCallEvent;

pub struct ToolCallPipeline {
    buffer: ToolCallBuffer,
    chat_message_parser: ChatMessageParser,
    validator: ToolCallValidator,
}

impl ToolCallPipeline {
    #[must_use]
    pub fn new(chat_message_parser: ChatMessageParser, validator: ToolCallValidator) -> Self {
        Self {
            buffer: ToolCallBuffer::default(),
            chat_message_parser,
            validator,
        }
    }

    pub fn feed(&mut self, fragment: &str) {
        self.buffer.append(fragment);
    }

    #[must_use]
    pub const fn buffer_is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn finalize(&mut self) -> ToolCallEvent {
        let input = self.buffer.take();

        if input.is_empty() {
            return ToolCallEvent::Resolved(Vec::new());
        }

        match self.chat_message_parser.parse(&input, false) {
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

    fn validate_resolved(&self, tool_calls: Vec<ParsedToolCall>) -> ToolCallEvent {
        let validation_errors: Vec<_> = tool_calls
            .iter()
            .filter_map(|tool_call| self.validator.validate(tool_call).err())
            .collect();

        if validation_errors.is_empty() {
            ToolCallEvent::Resolved(tool_calls)
        } else {
            ToolCallEvent::ValidationFailed(validation_errors)
        }
    }
}
