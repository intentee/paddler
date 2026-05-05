use llama_cpp_bindings::ParseChatMessageError;

/// Why a `ToolCallParser::parse` call failed. Bindings-side errors propagate
/// verbatim so callers can decide whether to retry, surface to clients, or
/// log and continue. Empty-input is its own variant — callers asking for a
/// parse on an empty buffer almost always indicate a state-machine bug.
#[derive(Debug, thiserror::Error)]
pub enum ToolCallParseError {
    #[error("tool-call parser invoked on empty buffer")]
    EmptyInput,
    #[error("bindings parse failed: {0}")]
    Bindings(#[from] ParseChatMessageError),
    #[error("could not serialize tools to JSON: {0}")]
    ToolsSerialization(String),
}
