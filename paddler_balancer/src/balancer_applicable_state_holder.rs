use std::sync::Arc;

use tokio::sync::watch;

use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_mode::InferenceMode;

use crate::balancer_applicable_state::BalancerApplicableState;
use crate::cluster_serves_another_inference_mode::ClusterServesAnotherInferenceMode;

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
    pub fn get_balancer_desired_state(&self) -> BalancerDesiredState {
        self.balancer_applicable_state_rx
            .borrow()
            .balancer_desired_state
            .clone()
    }

    pub fn require_inference_mode(
        &self,
        requested_inference_mode: InferenceMode,
    ) -> Result<Arc<BalancerApplicableState>, ClusterServesAnotherInferenceMode> {
        let balancer_applicable_state = self.balancer_applicable_state_rx.borrow().clone();
        let served_inference_mode = balancer_applicable_state.inference_mode();

        if served_inference_mode == requested_inference_mode {
            Ok(balancer_applicable_state)
        } else {
            Err(ClusterServesAnotherInferenceMode {
                requested_inference_mode,
                served_inference_mode,
            })
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

    use paddler_messaging::balancer_desired_state::BalancerDesiredState;
    use paddler_messaging::inference_mode::InferenceMode;

    use super::BalancerApplicableStateHolder;
    use crate::balancer_applicable_state::BalancerApplicableState;
    use crate::cluster_serves_another_inference_mode::ClusterServesAnotherInferenceMode;

    #[test]
    fn requires_the_inference_mode_the_applied_state_serves() {
        let (_balancer_applicable_state_tx, balancer_applicable_state_rx) = watch::channel(
            Arc::new(BalancerApplicableState::from(BalancerDesiredState {
                inference_mode: InferenceMode::Embeddings,
                ..BalancerDesiredState::default()
            })),
        );
        let holder = BalancerApplicableStateHolder::new(balancer_applicable_state_rx);

        assert_eq!(
            holder
                .require_inference_mode(InferenceMode::Embeddings)
                .map(|balancer_applicable_state| balancer_applicable_state.inference_mode()),
            Ok(InferenceMode::Embeddings)
        );
        assert_eq!(
            holder
                .require_inference_mode(InferenceMode::TextGeneration)
                .err(),
            Some(ClusterServesAnotherInferenceMode {
                requested_inference_mode: InferenceMode::TextGeneration,
                served_inference_mode: InferenceMode::Embeddings,
            })
        );
    }
}
