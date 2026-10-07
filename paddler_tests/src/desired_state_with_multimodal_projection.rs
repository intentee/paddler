use anyhow::Result;
use anyhow::bail;

use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::balancer_inference_settings::BalancerInferenceSettings;
use paddler_messaging::balancer_text_generation_settings::BalancerTextGenerationSettings;
use paddler_messaging::multimodal_settings::MultimodalSettings;

pub fn desired_state_with_multimodal_projection(
    desired_state: BalancerDesiredState,
    projection: AgentDesiredModel,
) -> Result<BalancerDesiredState> {
    let BalancerInferenceSettings::TextGeneration(text_generation_settings) =
        desired_state.inference_settings
    else {
        bail!("only a text generation desired state loads a multimodal projection");
    };

    Ok(BalancerDesiredState {
        inference_settings: BalancerInferenceSettings::TextGeneration(
            BalancerTextGenerationSettings {
                multimodal: MultimodalSettings {
                    projection,
                    ..text_generation_settings.multimodal.clone()
                },
                ..text_generation_settings
            },
        ),
        ..desired_state
    })
}
