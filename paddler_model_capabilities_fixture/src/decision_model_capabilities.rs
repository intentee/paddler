use llama_cpp_bindings_types::conversation_capability::ConversationCapability;
use llama_cpp_bindings_types::conversation_unsupported_reason::ConversationUnsupportedReason;
use llama_cpp_bindings_types::decision_capabilities::DecisionCapabilities;
use llama_cpp_bindings_types::decisions_capability::DecisionsCapability;
use llama_cpp_bindings_types::embeddings_capability::EmbeddingsCapability;
use llama_cpp_bindings_types::embeddings_unsupported_reason::EmbeddingsUnsupportedReason;
use llama_cpp_bindings_types::input_media_capabilities::InputMediaCapabilities;
use llama_cpp_bindings_types::model_capabilities::ModelCapabilities;
use llama_cpp_bindings_types::text_generation_capability::TextGenerationCapability;
use llama_cpp_bindings_types::text_generation_unsupported_reason::TextGenerationUnsupportedReason;

use crate::sampling_defaults_of_llama_cpp::model_defaults_of_llama_cpp;

/// A decision-only model that answers decisions from text and generates no text.
#[must_use]
pub const fn decision_model_capabilities() -> ModelCapabilities {
    ModelCapabilities {
        conversation: ConversationCapability::Unsupported(
            ConversationUnsupportedReason::ModelGeneratesNoText,
        ),
        decisions: DecisionsCapability::Supported(DecisionCapabilities {
            reads_images: false,
        }),
        embeddings: EmbeddingsCapability::Unsupported(
            EmbeddingsUnsupportedReason::ModelDeclaresNoVectorPooling,
        ),
        input_media: InputMediaCapabilities {
            audio: false,
            image: false,
        },
        model_defaults: model_defaults_of_llama_cpp(),
        text_generation: TextGenerationCapability::Unsupported(
            TextGenerationUnsupportedReason::NoTextOutput,
        ),
    }
}
