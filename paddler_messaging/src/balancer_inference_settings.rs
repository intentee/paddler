use serde::Deserialize;
use serde::Serialize;

use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;

use crate::balancer_text_generation_settings::BalancerTextGenerationSettings;
use crate::decision_settings::DecisionSettings;
use crate::inference_mode::InferenceMode;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum BalancerInferenceSettings {
    Decision(DecisionSettings),
    Embeddings(EmbeddingParameters),
    TextGeneration(BalancerTextGenerationSettings),
}

impl BalancerInferenceSettings {
    #[must_use]
    pub fn unconfigured(inference_mode: InferenceMode) -> Self {
        match inference_mode {
            InferenceMode::Decision => Self::Decision(DecisionSettings::default()),
            InferenceMode::Embeddings => Self::Embeddings(EmbeddingParameters::default()),
            InferenceMode::TextGeneration => {
                Self::TextGeneration(BalancerTextGenerationSettings::default())
            }
        }
    }

    #[must_use]
    pub const fn inference_mode(&self) -> InferenceMode {
        match self {
            Self::Decision(_) => InferenceMode::Decision,
            Self::Embeddings(_) => InferenceMode::Embeddings,
            Self::TextGeneration(_) => InferenceMode::TextGeneration,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BalancerInferenceSettings;
    use crate::inference_mode::InferenceMode;

    #[test]
    fn unconfigured_settings_serve_the_requested_mode() {
        for inference_mode in [
            InferenceMode::Decision,
            InferenceMode::Embeddings,
            InferenceMode::TextGeneration,
        ] {
            assert_eq!(
                BalancerInferenceSettings::unconfigured(inference_mode).inference_mode(),
                inference_mode
            );
        }
    }
}
