use crate::agent_desired_state::AgentDesiredState;

#[derive(Clone, Debug)]
pub struct BalancerApplicableState {
    pub agent_desired_state: AgentDesiredState,
}
