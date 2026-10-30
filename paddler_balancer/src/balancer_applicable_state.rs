use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;

#[derive(Clone, Debug)]
pub struct BalancerApplicableState {
    pub agent_desired_state: AgentDesiredState,
    pub balancer_desired_state: BalancerDesiredState,
    pub version: u64,
}

impl BalancerApplicableState {
    #[must_use]
    pub fn initial(balancer_desired_state: BalancerDesiredState) -> Self {
        Self::at_version(balancer_desired_state, 0)
    }

    #[must_use]
    pub fn superseded_by(&self, balancer_desired_state: BalancerDesiredState) -> Self {
        Self::at_version(balancer_desired_state, self.version + 1)
    }

    fn at_version(balancer_desired_state: BalancerDesiredState, version: u64) -> Self {
        Self {
            agent_desired_state: AgentDesiredState::from(&balancer_desired_state),
            balancer_desired_state,
            version,
        }
    }
}
