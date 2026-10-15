use serde::Deserialize;
use serde::Serialize;

use paddler_inference_parameters::pooling_type::PoolingType;

use crate::balancer_text_generation_settings::BalancerTextGenerationSettings;
use crate::decision_settings::DecisionSettings;
use crate::inference_mode::InferenceMode;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum AgentInferenceSettings {
    Decision(DecisionSettings),
    Embeddings(PoolingType),
    TextGeneration(BalancerTextGenerationSettings),
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
