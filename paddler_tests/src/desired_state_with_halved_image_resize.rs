use std::num::NonZeroU32;

use anyhow::Result;
use anyhow::bail;

use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::balancer_inference_settings::BalancerInferenceSettings;
use paddler_messaging::balancer_text_generation_settings::BalancerTextGenerationSettings;
use paddler_messaging::multimodal_settings::MultimodalSettings;

pub fn desired_state_with_halved_image_resize(
    desired_state: BalancerDesiredState,
) -> Result<BalancerDesiredState> {
    let BalancerInferenceSettings::TextGeneration(text_generation_settings) =
        desired_state.inference_settings
    else {
        bail!("only a text generation desired state resizes images");
    };

    Ok(BalancerDesiredState {
        inference_settings: BalancerInferenceSettings::TextGeneration(
            BalancerTextGenerationSettings {
                multimodal: MultimodalSettings {
                    image_resize_to_fit: NonZeroU32::try_from(
                        text_generation_settings
                            .multimodal
                            .image_resize_to_fit
                            .get()
                            / 2,
                    )?,
                    ..text_generation_settings.multimodal.clone()
                },
                ..text_generation_settings
            },
        ),
        ..desired_state
    })
}
