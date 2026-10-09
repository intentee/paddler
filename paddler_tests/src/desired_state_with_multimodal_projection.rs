use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::balancer_text_generation_settings::BalancerTextGenerationSettings;
use paddler_messaging::multimodal_settings::MultimodalSettings;

#[must_use]
pub fn desired_state_with_multimodal_projection(
    desired_state: BalancerDesiredState,
    projection: AgentDesiredModel,
) -> BalancerDesiredState {
    BalancerDesiredState {
        text_generation: BalancerTextGenerationSettings {
            multimodal: MultimodalSettings {
                projection,
                ..desired_state.text_generation.multimodal
            },
            ..desired_state.text_generation
        },
        ..desired_state
    }
}
