use std::num::NonZeroU32;

use anyhow::Result;

use paddler_inference_parameters::inference_parameters::InferenceParameters;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;

pub fn desired_state_with_halved_image_resize(
    desired_state: BalancerDesiredState,
) -> Result<BalancerDesiredState> {
    Ok(BalancerDesiredState {
        inference_parameters: InferenceParameters {
            image_resize_to_fit: NonZeroU32::try_from(
                desired_state.inference_parameters.image_resize_to_fit.get() / 2,
            )?,
            ..desired_state.inference_parameters.clone()
        },
        ..desired_state
    })
}
