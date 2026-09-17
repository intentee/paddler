use llama_cpp_bindings::ParseChatMessageError;
use llama_cpp_bindings::error::MarkerDetectionError;

#[derive(Debug, thiserror::Error)]
pub enum ToolCallPipelineError {
    #[error("tool-call pipeline invoked on empty buffer")]
    EmptyBuffer,
    #[error("bindings parse failed: {0}")]
    Bindings(#[from] ParseChatMessageError),
    #[error("the model's synthetic tool-call renders could not be diagnosed: {0}")]
    SyntheticRenderDiagnosisFailed(#[source] MarkerDetectionError),
}
