use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_mode::InferenceMode;

#[must_use]
pub fn desired_state_serving(inference_mode: InferenceMode) -> BalancerDesiredState {
    BalancerDesiredState {
        inference_mode,
        ..BalancerDesiredState::default()
    }
}
