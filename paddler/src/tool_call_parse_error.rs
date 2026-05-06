use llama_cpp_bindings::ParseChatMessageError;

#[derive(Debug, thiserror::Error)]
pub enum ToolCallParseError {
    #[error("tool-call parser invoked on empty buffer")]
    EmptyInput,
    #[error("bindings parse failed: {0}")]
    Bindings(#[from] ParseChatMessageError),
    #[error("template-override parser failed: {0}")]
    TemplateOverride(String),
    #[error("could not serialize tools to JSON: {0}")]
    ToolsSerialization(String),
}
