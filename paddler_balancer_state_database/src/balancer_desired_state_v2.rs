use serde::Deserialize;
use serde::de::IgnoredAny;

use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::balancer_text_generation_settings::BalancerTextGenerationSettings;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BalancerDesiredStateV2 {
    decision: IgnoredAny,
    embeddings: EmbeddingParameters,
    model: AgentDesiredModel,
    model_runtime_parameters: ModelRuntimeParameters,
    text_generation: BalancerTextGenerationSettings,
}

impl From<BalancerDesiredStateV2> for BalancerDesiredState {
    fn from(
        BalancerDesiredStateV2 {
            decision: IgnoredAny,
            embeddings,
            model,
            model_runtime_parameters,
            text_generation,
        }: BalancerDesiredStateV2,
    ) -> Self {
        Self {
            embeddings,
            model,
            model_runtime_parameters,
            text_generation,
        }
    }
}
