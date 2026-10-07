use anyhow::Result;
use anyhow::bail;

use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::balancer_inference_settings::BalancerInferenceSettings;
use paddler_messaging::balancer_text_generation_settings::BalancerTextGenerationSettings;
use paddler_messaging::chat_template::ChatTemplate;

pub fn desired_state_with_chat_template_override(
    desired_state: BalancerDesiredState,
    chat_template: ChatTemplate,
) -> Result<BalancerDesiredState> {
    let BalancerInferenceSettings::TextGeneration(text_generation_settings) =
        desired_state.inference_settings
    else {
        bail!("only a text generation desired state overrides the chat template");
    };

    Ok(BalancerDesiredState {
        inference_settings: BalancerInferenceSettings::TextGeneration(
            BalancerTextGenerationSettings {
                chat_template_override: Some(chat_template),
                use_chat_template_override: true,
                ..text_generation_settings
            },
        ),
        ..desired_state
    })
}
