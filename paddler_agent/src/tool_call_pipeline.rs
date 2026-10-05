use std::sync::Arc;

use llama_cpp_bindings::ChatMessageParseOutcome;
use llama_cpp_bindings::ParsedToolCall;
use llama_cpp_bindings::RawChatMessage;
use llama_cpp_bindings::model::LlamaModel;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::raw_tool_call_tokens::RawToolCallTokens;

use crate::tool_call_buffer::ToolCallBuffer;
use crate::tool_call_event::ToolCallEvent;
use crate::tool_call_pipeline_error::ToolCallPipelineError;
use crate::tool_call_validator::ToolCallValidator;

pub struct ToolCallPipeline {
    buffer: ToolCallBuffer,
    model: Arc<LlamaModel>,
    tools_json: Arc<str>,
    validator: ToolCallValidator,
}

impl ToolCallPipeline {
    pub fn new(
        model: Arc<LlamaModel>,
        tools: &[serde_json::Value],
        validator: ToolCallValidator,
    ) -> Result<Self, serde_json::Error> {
        let tools_json = Arc::from(serde_json::to_string(tools)?);

        Ok(Self {
            buffer: ToolCallBuffer::new(),
            model,
            tools_json,
            validator,
        })
    }

    pub fn feed(&mut self, fragment: &str) {
        self.buffer.append(fragment);
    }

    pub fn finalize(&mut self) -> ToolCallEvent {
        let input = self.buffer.take();
        if input.is_empty() {
            return ToolCallEvent::Resolved(Vec::new());
        }

        match self
            .model
            .parse_chat_message(&self.tools_json, &input, false)
        {
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
            Err(err) => ToolCallEvent::ParseFailed(ToolCallPipelineError::Bindings(err)),
        }
    }

    pub fn finalize_to_generated_event(&mut self) -> Option<GeneratedTokenResult> {
        self.finalize().into_generated_token_result()
    }

    #[must_use]
    pub const fn buffer_is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    fn validate_resolved(&self, tool_calls: Vec<ParsedToolCall>) -> ToolCallEvent {
        let mut errors = Vec::new();
        for call in &tool_calls {
            if let Err(err) = self.validator.validate(call) {
                errors.push(err);
            }
        }

        if errors.is_empty() {
            ToolCallEvent::Resolved(tool_calls)
        } else {
            ToolCallEvent::ValidationFailed(errors)
        }
    }
}
