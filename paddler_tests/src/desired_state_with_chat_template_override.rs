use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::balancer_text_generation_settings::BalancerTextGenerationSettings;
use paddler_messaging::chat_template::ChatTemplate;

#[must_use]
pub fn desired_state_with_chat_template_override(
    desired_state: BalancerDesiredState,
    chat_template: ChatTemplate,
) -> BalancerDesiredState {
    BalancerDesiredState {
        text_generation: BalancerTextGenerationSettings {
            chat_template_override: Some(chat_template),
            use_chat_template_override: true,
            ..desired_state.text_generation
        },
        ..desired_state
    }
}
