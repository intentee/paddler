use serde::Deserialize;
use serde::Serialize;

use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;

use crate::agent_text_generation_settings::AgentTextGenerationSettings;
use crate::decision_settings::DecisionSettings;
use crate::inference_mode::InferenceMode;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum AgentInferenceSettings {
    Decision(DecisionSettings),
    Embeddings(EmbeddingParameters),
    TextGeneration(AgentTextGenerationSettings),
}

impl AgentInferenceSettings {
    #[must_use]
    pub const fn inference_mode(&self) -> InferenceMode {
        match self {
            Self::Decision(_) => InferenceMode::Decision,
            Self::Embeddings(_) => InferenceMode::Embeddings,
            Self::TextGeneration(_) => InferenceMode::TextGeneration,
        }
    }
}
