use llama_cpp_bindings::ParseChatMessageError;

#[derive(Debug, thiserror::Error)]
pub enum ToolCallPipelineError {
    #[error("tool-call pipeline invoked on empty buffer")]
    EmptyBuffer,
    #[error("bindings parse failed: {0}")]
    Bindings(#[from] ParseChatMessageError),
}
