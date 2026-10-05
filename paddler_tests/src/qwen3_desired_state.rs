use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_parameters::InferenceParameters;

use crate::model_card::ModelCard;
use crate::model_card::qwen3_0_6b::qwen3_0_6b;

#[must_use]
pub fn qwen3_desired_state() -> BalancerDesiredState {
    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_0_6b();

    BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters {
            n_gpu_layers: gpu_layer_count,
            ..InferenceParameters::deterministic()
        },
        model: AgentDesiredModel::HuggingFace(reference),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: false,
    }
}
