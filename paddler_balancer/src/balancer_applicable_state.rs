use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;

#[derive(Clone, Debug)]
pub struct BalancerApplicableState {
    pub agent_desired_state: AgentDesiredState,
}

impl From<BalancerDesiredState> for BalancerApplicableState {
    fn from(balancer_desired_state: BalancerDesiredState) -> Self {
        Self {
            agent_desired_state: AgentDesiredState::from(balancer_desired_state),
        }
    }
}
