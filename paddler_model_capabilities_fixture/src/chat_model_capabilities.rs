use llama_cpp_bindings_types::chat_template_capabilities::ChatTemplateCapabilities;
use llama_cpp_bindings_types::conversation_capability::ConversationCapability;
use llama_cpp_bindings_types::decisions_capability::DecisionsCapability;
use llama_cpp_bindings_types::decisions_unsupported_reason::DecisionsUnsupportedReason;
use llama_cpp_bindings_types::embeddings_capability::EmbeddingsCapability;
use llama_cpp_bindings_types::embeddings_unsupported_reason::EmbeddingsUnsupportedReason;
use llama_cpp_bindings_types::input_media_capabilities::InputMediaCapabilities;
use llama_cpp_bindings_types::model_capabilities::ModelCapabilities;
use llama_cpp_bindings_types::text_generation_capability::TextGenerationCapability;
use llama_cpp_bindings_types::tool_calling_capability::ToolCallingCapability;

use crate::sampling_defaults_of_llama_cpp::model_defaults_of_llama_cpp;

/// A chat model that converses with thinking and tool calls, and serves neither embeddings nor
/// decisions.
#[must_use]
pub const fn chat_model_capabilities() -> ModelCapabilities {
    ModelCapabilities {
        conversation: ConversationCapability::Supported(ChatTemplateCapabilities {
            system_role: true,
            thinking: true,
            tool_calling: ToolCallingCapability::ParallelCalls,
        }),
        decisions: DecisionsCapability::Unsupported(
            DecisionsUnsupportedReason::ModelIsNotADecisionModel,
        ),
        embeddings: EmbeddingsCapability::Unsupported(
            EmbeddingsUnsupportedReason::ModelDeclaresNoVectorPooling,
        ),
        input_media: InputMediaCapabilities {
            audio: false,
            image: false,
        },
        model_defaults: model_defaults_of_llama_cpp(),
        text_generation: TextGenerationCapability::Supported,
    }
}
