use std::sync::RwLock;

use crate::agent_desired_state::AgentDesiredState;
use crate::balancer_applicable_state::BalancerApplicableState;

pub struct BalancerApplicableStateHolder {
    balancer_applicable_state: RwLock<Option<BalancerApplicableState>>,
}

impl BalancerApplicableStateHolder {
    pub fn get_agent_desired_state(&self) -> Option<AgentDesiredState> {
        self.balancer_applicable_state
            .read()
            .expect("Failed to get balancer state lock")
            .as_ref()
            .map(|state| state.agent_desired_state.clone())
    }

    pub fn set_balancer_applicable_state(
        &self,
        balancer_applicable_state: Option<BalancerApplicableState>,
    ) {
        let mut lock = self
            .balancer_applicable_state
            .write()
            .expect("Failed to get balancer state lock");

        *lock = balancer_applicable_state;
    }
}

impl Default for BalancerApplicableStateHolder {
    fn default() -> Self {
        Self {
            balancer_applicable_state: RwLock::new(None),
        }
    }
}
