use parking_lot::RwLock;
use tokio::sync::watch;

use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_messaging::subscribes_to_updates::SubscribesToUpdates;

use crate::balancer_applicable_state::BalancerApplicableState;
use crate::cluster_serves_another_inference_mode::ClusterServesAnotherInferenceMode;

pub struct BalancerApplicableStateHolder {
    balancer_applicable_state: RwLock<BalancerApplicableState>,
    update_tx: watch::Sender<()>,
}

impl BalancerApplicableStateHolder {
    #[must_use]
    pub fn new(balancer_applicable_state: BalancerApplicableState) -> Self {
        let (update_tx, _initial_rx) = watch::channel(());

        Self {
            balancer_applicable_state: RwLock::new(balancer_applicable_state),
            update_tx,
        }
    }

    pub fn get_agent_desired_state(&self) -> AgentDesiredState {
        self.balancer_applicable_state
            .read()
            .agent_desired_state
            .clone()
    }

    pub fn get_balancer_applicable_state(&self) -> BalancerApplicableState {
        self.balancer_applicable_state.read().clone()
    }

    #[must_use]
    pub fn inference_mode(&self) -> InferenceMode {
        self.balancer_applicable_state
            .read()
            .agent_desired_state
            .inference_settings
            .inference_mode()
    }

    pub fn require_inference_mode(
        &self,
        requested_inference_mode: InferenceMode,
    ) -> Result<(), ClusterServesAnotherInferenceMode> {
        let served_inference_mode = self.inference_mode();

        if served_inference_mode == requested_inference_mode {
            Ok(())
        } else {
            Err(ClusterServesAnotherInferenceMode {
                requested_inference_mode,
                served_inference_mode,
            })
        }
    }

    pub fn set_balancer_applicable_state(
        &self,
        balancer_applicable_state: BalancerApplicableState,
    ) {
        *self.balancer_applicable_state.write() = balancer_applicable_state;

        self.update_tx.send_replace(());
    }
}

impl SubscribesToUpdates for BalancerApplicableStateHolder {
    fn subscribe_to_updates(&self) -> watch::Receiver<()> {
        self.update_tx.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;
    use paddler_messaging::inference_mode::InferenceMode;
    use paddler_messaging::subscribes_to_updates::SubscribesToUpdates as _;

    use super::BalancerApplicableStateHolder;
    use crate::balancer_applicable_state::BalancerApplicableState;
    use crate::cluster_serves_another_inference_mode::ClusterServesAnotherInferenceMode;

    fn holder_serving(inference_mode: InferenceMode) -> BalancerApplicableStateHolder {
        BalancerApplicableStateHolder::new(BalancerApplicableState::from(BalancerDesiredState {
            inference_mode,
            ..BalancerDesiredState::default()
        }))
    }

    #[test]
    fn replacing_the_state_notifies_subscribers_and_is_read_back() {
        let holder = holder_serving(InferenceMode::TextGeneration);
        let update_rx = holder.subscribe_to_updates();

        holder.set_balancer_applicable_state(BalancerApplicableState::from(BalancerDesiredState {
            model: AgentDesiredModel::LocalToAgent("model.gguf".to_owned()),
            ..BalancerDesiredState::default()
        }));

        assert!(
            update_rx
                .has_changed()
                .expect("the holder must keep its update sender")
        );
        assert_eq!(
            holder.get_agent_desired_state().model,
            AgentDesiredModel::LocalToAgent("model.gguf".to_owned())
        );
    }

    #[test]
    fn requires_the_inference_mode_the_applied_state_serves() {
        let holder = holder_serving(InferenceMode::Embeddings);

        assert_eq!(
            holder.require_inference_mode(InferenceMode::Embeddings),
            Ok(())
        );
        assert_eq!(
            holder.require_inference_mode(InferenceMode::TextGeneration),
            Err(ClusterServesAnotherInferenceMode {
                requested_inference_mode: InferenceMode::TextGeneration,
                served_inference_mode: InferenceMode::Embeddings,
            })
        );
    }
}
