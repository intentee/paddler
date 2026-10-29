use serde::Deserialize;
use serde::Serialize;

use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_inference_parameters::pooling_type::PoolingType;

use crate::agent_decisions_settings::AgentDecisionsSettings;
use crate::agent_desired_model::AgentDesiredModel;
use crate::balancer_desired_state::BalancerDesiredState;
use crate::balancer_text_generation_settings::BalancerTextGenerationSettings;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgentDesiredState {
    pub decisions: AgentDecisionsSettings,
    pub model: AgentDesiredModel,
    pub model_runtime_parameters: ModelRuntimeParameters,
    pub pooling_type: PoolingType,
    pub text_generation: BalancerTextGenerationSettings,
}

impl Default for AgentDesiredState {
    fn default() -> Self {
        Self::from(&BalancerDesiredState::default())
    }
}

impl From<&BalancerDesiredState> for AgentDesiredState {
    fn from(
        BalancerDesiredState {
            decision,
            embeddings: EmbeddingParameters { pooling_type, .. },
            model,
            model_runtime_parameters,
            text_generation,
        }: &BalancerDesiredState,
    ) -> Self {
        Self {
            decisions: decision.agent_decisions(),
            model: model.clone(),
            model_runtime_parameters: model_runtime_parameters.clone(),
            pooling_type: pooling_type.clone(),
            text_generation: text_generation.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
    use paddler_inference_parameters::pooling_type::PoolingType;

    use super::AgentDesiredState;
    use crate::balancer_desired_state::BalancerDesiredState;

    #[test]
    fn the_balancer_only_embedding_batch_size_does_not_reach_agents() {
        let desired_state_batching = |embedding_batch_size| {
            AgentDesiredState::from(&BalancerDesiredState {
                embeddings: EmbeddingParameters {
                    embedding_batch_size,
                    pooling_type: PoolingType::Mean,
                },
                ..BalancerDesiredState::default()
            })
        };

        assert_eq!(
            desired_state_batching(NonZeroUsize::MIN),
            desired_state_batching(NonZeroUsize::MAX)
        );
    }
}
