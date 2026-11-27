use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;

use llama_cpp_bindings_types::conversation_capability::ConversationCapability;
use llama_cpp_bindings_types::decisions_capability::DecisionsCapability;
use llama_cpp_bindings_types::embeddings_capability::EmbeddingsCapability;
use llama_cpp_bindings_types::model_capabilities::ModelCapabilities;
use llama_cpp_bindings_types::text_generation_capability::TextGenerationCapability;
use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub enum RequestedCapability {
    Conversation,
    Decisions,
    Embeddings,
    RawTextGeneration,
}

impl RequestedCapability {
    #[must_use]
    pub const fn is_supported_by(
        self,
        ModelCapabilities {
            conversation,
            decisions,
            embeddings,
            text_generation,
            ..
        }: &ModelCapabilities,
    ) -> bool {
        match self {
            Self::Conversation => matches!(conversation, ConversationCapability::Supported(_)),
            Self::Decisions => matches!(decisions, DecisionsCapability::Supported(_)),
            Self::Embeddings => matches!(embeddings, EmbeddingsCapability::Supported),
            Self::RawTextGeneration => {
                matches!(text_generation, TextGenerationCapability::Supported)
            }
        }
    }
}

impl Display for RequestedCapability {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Conversation => "conversation",
            Self::Decisions => "decisions",
            Self::Embeddings => "embeddings",
            Self::RawTextGeneration => "raw text generation",
        })
    }
}
