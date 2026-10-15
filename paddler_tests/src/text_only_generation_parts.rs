use std::sync::Arc;

use llama_cpp_bindings::StreamingMarkers;

use paddler_agent_text_generation::chat_template_renderer::ChatTemplateRenderer;

pub struct TextOnlyGenerationParts {
    pub chat_template_renderer: ChatTemplateRenderer,
    pub sequence_context_size: u32,
    pub streaming_markers: Arc<StreamingMarkers>,
}
