use std::sync::Arc;

use tokio::sync::watch;

use paddler_messaging::agent_decisions_settings::AgentDecisionsSettings;
use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;

use crate::balancer_applicable_state::BalancerApplicableState;
use crate::cluster_serves_no_decisions::ClusterServesNoDecisions;

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

    pub fn require_decisions(&self) -> Result<(), ClusterServesNoDecisions> {
        match self
            .balancer_applicable_state_rx
            .borrow()
            .agent_desired_state
            .decisions
        {
            AgentDecisionsSettings::NotServed => Err(ClusterServesNoDecisions),
            AgentDecisionsSettings::Served(_) => Ok(()),
        }
    }

    #[must_use]
    pub fn subscribe(&self) -> watch::Receiver<Arc<BalancerApplicableState>> {
        self.balancer_applicable_state_rx.clone()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tokio::sync::watch;

    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;
    use paddler_messaging::decision_settings::DecisionSettings;

    use super::BalancerApplicableStateHolder;
    use crate::balancer_applicable_state::BalancerApplicableState;
    use crate::cluster_serves_no_decisions::ClusterServesNoDecisions;

    fn holder_of(pointer_head: AgentDesiredModel) -> BalancerApplicableStateHolder {
        let (_balancer_applicable_state_tx, balancer_applicable_state_rx) = watch::channel(
            Arc::new(BalancerApplicableState::initial(BalancerDesiredState {
                decision: DecisionSettings { pointer_head },
                ..BalancerDesiredState::default()
            })),
        );

        BalancerApplicableStateHolder::new(balancer_applicable_state_rx)
    }

    #[test]
    fn grants_decisions_while_a_pointer_head_is_configured() {
        assert_eq!(
            holder_of(AgentDesiredModel::Uri("pointer_head.gguf".to_owned())).require_decisions(),
            Ok(())
        );
    }

    #[test]
    fn refuses_decisions_while_no_pointer_head_is_configured() {
        assert_eq!(
            holder_of(AgentDesiredModel::None).require_decisions(),
            Err(ClusterServesNoDecisions)
        );
    }
}
