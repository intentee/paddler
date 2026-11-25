use std::sync::Arc;

use tokio::sync::watch;

use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;

use crate::balancer_applicable_state::BalancerApplicableState;

pub struct BalancerApplicableStateHolder {
    balancer_applicable_state_rx: watch::Receiver<Arc<BalancerApplicableState>>,
}

impl BalancerApplicableStateHolder {
    #[must_use]
    pub const fn new(
        balancer_applicable_state_rx: watch::Receiver<Arc<BalancerApplicableState>>,
    ) -> Self {
        Self {
            balancer_applicable_state_rx,
        }
    }

    #[must_use]
    pub fn get_agent_desired_state(&self) -> AgentDesiredState {
        self.balancer_applicable_state_rx
            .borrow()
            .agent_desired_state
            .clone()
    }

    #[must_use]
    pub fn get_balancer_applicable_state(&self) -> Arc<BalancerApplicableState> {
        self.balancer_applicable_state_rx.borrow().clone()
    }

    #[must_use]
    pub fn get_balancer_desired_state(&self) -> BalancerDesiredState {
        self.balancer_applicable_state_rx
            .borrow()
            .balancer_desired_state
            .clone()
    }

    #[must_use]
    pub fn subscribe(&self) -> watch::Receiver<Arc<BalancerApplicableState>> {
        self.balancer_applicable_state_rx.clone()
    }
}
