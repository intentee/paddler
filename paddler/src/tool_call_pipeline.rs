use paddler_types::parsed_tool_call::ParsedToolCall;

use crate::tool_call_buffer::ToolCallBuffer;
use crate::tool_call_event::ToolCallEvent;
use crate::tool_call_parser::ToolCallParser;
use crate::tool_call_validator::ToolCallValidator;

pub struct ToolCallPipeline {
    buffer: ToolCallBuffer,
    parser: ToolCallParser,
    validator: ToolCallValidator,
}

impl ToolCallPipeline {
    #[must_use]
    pub const fn new(parser: ToolCallParser, validator: ToolCallValidator) -> Self {
        Self {
            buffer: ToolCallBuffer::new(),
            parser,
            validator,
        }
    }

    pub fn feed(&mut self, fragment: &str) {
        self.buffer.append(fragment);
    }

    pub fn finalize(&mut self) -> ToolCallEvent {
        let input = self.buffer.take();
        if input.is_empty() {
            return ToolCallEvent::Resolved(Vec::new());
        }

        match self.parser.parse(&input) {
            Ok(parsed) => self.validate_resolved(parsed.tool_calls),
            Err(err) => ToolCallEvent::ParseFailed(err),
        }
    }

    #[must_use]
    pub fn try_partial(&self) -> ToolCallEvent {
        let input = self.buffer.as_str();
        if input.is_empty() {
            return ToolCallEvent::Pending;
        }

        match self.parser.parse_partial(input) {
            Ok(parsed) if parsed.tool_calls.is_empty() => ToolCallEvent::Pending,
            Ok(parsed) => self.validate_resolved(parsed.tool_calls),
            Err(err) => ToolCallEvent::ParseFailed(err),
        }
    }

    pub fn reset(&mut self) {
        self.buffer.clear();
    }

    #[must_use]
    pub const fn buffer_is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    fn validate_resolved(&self, tool_calls: Vec<ParsedToolCall>) -> ToolCallEvent {
        let parsed_with_ids: Vec<ParsedToolCall> = tool_calls
            .into_iter()
            .enumerate()
            .map(|(index, mut call)| {
                if call.id.is_empty() {
                    call.id = format!("call_{index}");
                }
                call
            })
            .collect();

        let mut errors = Vec::new();
        for call in &parsed_with_ids {
            if let Err(err) = self.validator.validate(call) {
                errors.push(err);
            }
        }

        if errors.is_empty() {
            ToolCallEvent::Resolved(parsed_with_ids)
        } else {
            ToolCallEvent::ValidationFailed(errors)
        }
    }
}
