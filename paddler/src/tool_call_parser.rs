use std::sync::Arc;

use llama_cpp_bindings::ParsedChatMessage;
use llama_cpp_bindings::model::LlamaModel;

use crate::tool_call_format;
use crate::tool_call_parse_error::ToolCallParseError;

#[derive(Clone)]
pub struct ToolCallParser {
    model: Arc<LlamaModel>,
    tools_json: Arc<str>,
}

impl ToolCallParser {
    pub fn new(
        model: Arc<LlamaModel>,
        tools: &[serde_json::Value],
    ) -> Result<Self, ToolCallParseError> {
        let tools_json = serde_json::to_string(tools)
            .map_err(|err| ToolCallParseError::ToolsSerialization(err.to_string()))?;

        Ok(Self {
            model,
            tools_json: Arc::from(tools_json),
        })
    }

    pub fn parse(&self, input: &str) -> Result<ParsedChatMessage, ToolCallParseError> {
        if input.is_empty() {
            return Err(ToolCallParseError::EmptyInput);
        }

        let mut parsed = self
            .model
            .parse_chat_message(&self.tools_json, input, false)?;

        if parsed.tool_calls.is_empty()
            && let Some(markers) = self.model.tool_call_markers()
        {
            let fallback = tool_call_format::try_parse(input, &markers)
                .map_err(|err| ToolCallParseError::TemplateOverride(err.to_string()))?;
            if !fallback.is_empty() {
                parsed.tool_calls = fallback;
            }
        }

        Ok(parsed)
    }

    pub fn parse_partial(&self, input: &str) -> Result<ParsedChatMessage, ToolCallParseError> {
        if input.is_empty() {
            return Err(ToolCallParseError::EmptyInput);
        }

        Ok(self
            .model
            .parse_chat_message(&self.tools_json, input, true)?)
    }

    #[must_use]
    pub fn tools_json(&self) -> &str {
        &self.tools_json
    }
}
