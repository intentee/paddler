use paddler_types::parsed_tool_call::ParsedToolCall;

use crate::tool_call_buffer::ToolCallBuffer;
use crate::tool_call_event::ToolCallEvent;
use crate::tool_call_parser::ToolCallParser;
use crate::tool_call_validator::ToolCallValidator;

/// Stateful tool-call pipeline shared by both the OpenAI compat endpoint and
/// the internal endpoints.
///
/// Composition only — every responsibility lives in a sibling module:
/// - [`ToolCallBuffer`] accumulates fragments.
/// - [`ToolCallParser`] turns the buffered text into structured calls via
///   the bindings.
/// - [`ToolCallValidator`] checks each call against the request's tool
///   schemas (or the JSON-object fallback when no schema is declared).
///
/// The validator is **always** consulted: callers don't get to skip
/// validation, they get to choose between schema mode and JSON-object mode
/// at validator construction time.
pub struct ToolCallPipeline {
    buffer: ToolCallBuffer,
    parser: ToolCallParser,
    validator: ToolCallValidator,
}

impl ToolCallPipeline {
    #[must_use]
    pub fn new(parser: ToolCallParser, validator: ToolCallValidator) -> Self {
        Self {
            buffer: ToolCallBuffer::new(),
            parser,
            validator,
        }
    }

    /// Append a streamed tool-call text fragment to the internal buffer.
    pub fn feed(&mut self, fragment: &str) {
        self.buffer.append(fragment);
    }

    /// Parse + validate the accumulated buffer, then clear it.
    ///
    /// Always returns one of `Resolved`, `ParseFailed`, or
    /// `ValidationFailed`. `Pending` is reserved for `try_partial`.
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

    /// Peek at the buffer with a partial-tolerant parse. Does NOT clear the
    /// buffer. Useful for emitting an intermediate "we have a name" event
    /// once enough text is buffered, while still letting the final
    /// `finalize` produce the canonical resolved event.
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
    pub fn buffer_is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    fn validate_resolved(
        &self,
        tool_calls: Vec<llama_cpp_bindings::ParsedToolCall>,
    ) -> ToolCallEvent {
        let parsed_wire: Vec<ParsedToolCall> = tool_calls
            .into_iter()
            .map(|call| ParsedToolCall::new(call.id, call.name, call.arguments_json))
            .collect();

        let mut errors = Vec::new();
        for call in &parsed_wire {
            if let Err(err) = self.validator.validate(call) {
                errors.push(err);
            }
        }

        if errors.is_empty() {
            ToolCallEvent::Resolved(parsed_wire)
        } else {
            ToolCallEvent::ValidationFailed(errors)
        }
    }
}

// Pipeline composition needs a real LlamaModel to exercise; integration tests
// live under paddler_tests/tests/qwen3_*. The constituent units —
// tool_call_buffer, tool_call_validator, and tool_call_event — each carry
// their own unit tests independent of the model.
