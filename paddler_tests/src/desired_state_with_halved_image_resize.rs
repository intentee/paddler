use std::num::NonZeroU32;

use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::balancer_text_generation_settings::BalancerTextGenerationSettings;
use paddler_messaging::multimodal_settings::MultimodalSettings;

const HALF: NonZeroU32 = NonZeroU32::new(2).unwrap();

#[must_use]
pub fn desired_state_with_halved_image_resize(
    desired_state: BalancerDesiredState,
) -> BalancerDesiredState {
    BalancerDesiredState {
        text_generation: BalancerTextGenerationSettings {
            multimodal: MultimodalSettings {
                image_resize_to_fit: desired_state
                    .text_generation
                    .multimodal
                    .image_resize_to_fit
                    .div_ceil(HALF),
                ..desired_state.text_generation.multimodal
            },
            ..desired_state.text_generation
        },
        ..desired_state
    }
}
