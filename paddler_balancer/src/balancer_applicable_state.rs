use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_mode::InferenceMode;

#[derive(Clone, Debug)]
pub struct BalancerApplicableState {
    pub agent_desired_state: AgentDesiredState,
    pub balancer_desired_state: BalancerDesiredState,
}

impl BalancerApplicableState {
    #[must_use]
    pub const fn inference_mode(&self) -> InferenceMode {
        self.agent_desired_state.inference_settings.inference_mode()
    }
}

impl From<BalancerDesiredState> for BalancerApplicableState {
    fn from(balancer_desired_state: BalancerDesiredState) -> Self {
        Self {
            agent_desired_state: AgentDesiredState::from(balancer_desired_state.clone()),
            balancer_desired_state,
        }
    }
}
