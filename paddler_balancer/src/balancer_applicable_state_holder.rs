use parking_lot::RwLock;
use tokio::sync::watch;

use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::subscribes_to_updates::SubscribesToUpdates;

use crate::balancer_applicable_state::BalancerApplicableState;

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

    #[test]
    fn replacing_the_state_notifies_subscribers_and_is_read_back() {
        let holder = BalancerApplicableStateHolder::new(BalancerApplicableState::from(
            BalancerDesiredState::unconfigured(InferenceMode::TextGeneration),
        ));
        let update_rx = holder.subscribe_to_updates();

        holder.set_balancer_applicable_state(BalancerApplicableState::from(BalancerDesiredState {
            model: AgentDesiredModel::LocalToAgent("model.gguf".to_owned()),
            ..BalancerDesiredState::unconfigured(InferenceMode::TextGeneration)
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
}
