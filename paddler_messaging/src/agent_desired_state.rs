use serde::Deserialize;
use serde::Serialize;

use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;

use crate::agent_desired_model::AgentDesiredModel;
use crate::agent_inference_settings::AgentInferenceSettings;
use crate::agent_text_generation_settings::AgentTextGenerationSettings;
use crate::balancer_desired_state::BalancerDesiredState;
use crate::inference_mode::InferenceMode;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgentDesiredState {
    pub inference_settings: AgentInferenceSettings,
    pub model: AgentDesiredModel,
    pub model_runtime_parameters: ModelRuntimeParameters,
}

impl From<BalancerDesiredState> for AgentDesiredState {
    fn from(
        BalancerDesiredState {
            decision,
            embeddings,
            inference_mode,
            model,
            model_runtime_parameters,
            text_generation,
        }: BalancerDesiredState,
    ) -> Self {
        Self {
            inference_settings: match inference_mode {
                InferenceMode::Decision => AgentInferenceSettings::Decision(decision),
                InferenceMode::Embeddings => AgentInferenceSettings::Embeddings(embeddings),
                InferenceMode::TextGeneration => AgentInferenceSettings::TextGeneration(
                    AgentTextGenerationSettings::from(text_generation),
                ),
            },
            model,
            model_runtime_parameters,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
    use paddler_inference_parameters::pooling_type::PoolingType;
    use paddler_inference_parameters::sampling_parameters::SamplingParameters;

    use super::AgentDesiredState;
    use crate::agent_desired_model::AgentDesiredModel;
    use crate::agent_inference_settings::AgentInferenceSettings;
    use crate::agent_text_generation_settings::AgentTextGenerationSettings;
    use crate::balancer_desired_state::BalancerDesiredState;
    use crate::balancer_text_generation_settings::BalancerTextGenerationSettings;
    use crate::decision_settings::DecisionSettings;
    use crate::inference_mode::InferenceMode;

    fn decision_settings() -> DecisionSettings {
        DecisionSettings {
            pointer_head: AgentDesiredModel::Uri("pointer_head.gguf".to_owned()),
        }
    }

    fn embedding_parameters() -> EmbeddingParameters {
        EmbeddingParameters {
            embedding_batch_size: NonZeroUsize::MIN,
            pooling_type: PoolingType::Mean,
        }
    }

    fn text_generation_settings() -> BalancerTextGenerationSettings {
        BalancerTextGenerationSettings {
            sampling_parameters: SamplingParameters::deterministic(),
            ..BalancerTextGenerationSettings::default()
        }
    }

    fn agent_settings_while_serving(inference_mode: InferenceMode) -> AgentInferenceSettings {
        AgentDesiredState::from(BalancerDesiredState {
            decision: decision_settings(),
            embeddings: embedding_parameters(),
            inference_mode,
            text_generation: text_generation_settings(),
            ..BalancerDesiredState::default()
        })
        .inference_settings
    }

    #[test]
    fn agents_receive_only_the_settings_of_the_active_inference_mode() {
        assert_eq!(
            agent_settings_while_serving(InferenceMode::Decision),
            AgentInferenceSettings::Decision(decision_settings())
        );
        assert_eq!(
            agent_settings_while_serving(InferenceMode::Embeddings),
            AgentInferenceSettings::Embeddings(embedding_parameters())
        );
        assert_eq!(
            agent_settings_while_serving(InferenceMode::TextGeneration),
            AgentInferenceSettings::TextGeneration(AgentTextGenerationSettings::from(
                text_generation_settings()
            ))
        );
    }
}
