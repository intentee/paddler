use std::sync::Arc;

use llama_cpp_bindings::StreamingMarkers;

use crate::chat_prompt_renderer::ChatPromptRenderer;

pub struct TokenGenerationSupport {
    pub chat_prompt_renderer: ChatPromptRenderer,
    pub streaming_markers: Arc<StreamingMarkers>,
}
