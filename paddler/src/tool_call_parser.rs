use std::sync::Arc;

use llama_cpp_bindings::ParsedChatMessage;
use llama_cpp_bindings::model::LlamaModel;

use crate::tool_call_parse_error::ToolCallParseError;

/// Thin wrapper around `LlamaModel::parse_chat_message`. Owns the tools
/// payload (pre-serialized to JSON once at construction) and the model
/// handle so callers can treat parsing as a pure function of the buffered
/// input string.
///
/// Parsing happens entirely in the bindings/C++ side via llama.cpp's
/// `common_chat_parse`. This struct never deserializes JSON in Rust on
/// model output.
#[derive(Clone)]
pub struct ToolCallParser {
    model: Arc<LlamaModel>,
    tools_json: Arc<str>,
}

impl ToolCallParser {
    /// Build a parser bound to the given model and tools array. `tools` is
    /// expected to be a JSON-serializable list of OpenAI-style tool
    /// definitions; an empty slice serializes to `"[]"` and tells the
    /// underlying parser to refuse tool calls entirely.
    ///
    /// # Errors
    /// Returns [`ToolCallParseError::ToolsSerialization`] when serde_json
    /// cannot serialize the supplied tools array (in practice only on
    /// non-string map keys, which the workspace types disallow upstream).
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

    /// Parse a complete tool-call payload (close marker has been seen).
    ///
    /// # Errors
    /// Returns [`ToolCallParseError::EmptyInput`] when called with no buffer
    /// content, or [`ToolCallParseError::Bindings`] when the FFI raises.
    pub fn parse(&self, input: &str) -> Result<ParsedChatMessage, ToolCallParseError> {
        if input.is_empty() {
            return Err(ToolCallParseError::EmptyInput);
        }

        Ok(self.model.parse_chat_message(&self.tools_json, input, false)?)
    }

    /// Parse a partial tool-call payload (still mid-stream). Lenient — the
    /// underlying parser tolerates incomplete input.
    ///
    /// # Errors
    /// Returns [`ToolCallParseError::EmptyInput`] or [`ToolCallParseError::Bindings`].
    pub fn parse_partial(
        &self,
        input: &str,
    ) -> Result<ParsedChatMessage, ToolCallParseError> {
        if input.is_empty() {
            return Err(ToolCallParseError::EmptyInput);
        }

        Ok(self.model.parse_chat_message(&self.tools_json, input, true)?)
    }

    #[must_use]
    pub fn tools_json(&self) -> &str {
        &self.tools_json
    }
}
