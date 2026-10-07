use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_mode::InferenceMode;

pub enum ClusterDesiredState {
    Apply(Box<BalancerDesiredState>),
    KeepStored(InferenceMode),
}

impl ClusterDesiredState {
    #[must_use]
    pub const fn inference_mode(&self) -> InferenceMode {
        match self {
            Self::Apply(balancer_desired_state) => {
                balancer_desired_state.inference_settings.inference_mode()
            }
            Self::KeepStored(inference_mode) => *inference_mode,
        }
    }
}
