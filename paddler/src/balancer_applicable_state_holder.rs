use crate::agent_desired_state::AgentDesiredState;
use parking_lot::RwLock;
use tokio::sync::watch;

use crate::balancer_applicable_state::BalancerApplicableState;
use crate::subscribes_to_updates::SubscribesToUpdates;

pub struct BalancerApplicableStateHolder {
    update_tx: watch::Sender<()>,
    balancer_applicable_state: RwLock<Option<BalancerApplicableState>>,
}

impl BalancerApplicableStateHolder {
    pub fn get_agent_desired_state(&self) -> Option<AgentDesiredState> {
        self.balancer_applicable_state
            .read()
            .as_ref()
            .map(|state| state.agent_desired_state.clone())
    }

    pub fn get_balancer_applicable_state(&self) -> Option<BalancerApplicableState> {
        self.balancer_applicable_state.read().clone()
    }

    pub fn set_balancer_applicable_state(
        &self,
        balancer_applicable_state: Option<BalancerApplicableState>,
    ) {
        {
            let mut lock = self.balancer_applicable_state.write();

            *lock = balancer_applicable_state;
        }

        self.update_tx.send_replace(());
    }
}

impl Default for BalancerApplicableStateHolder {
    fn default() -> Self {
        let (update_tx, _initial_rx) = watch::channel(());

        Self {
            balancer_applicable_state: RwLock::new(None),
            update_tx,
        }
    }
}

impl SubscribesToUpdates for BalancerApplicableStateHolder {
    fn subscribe_to_updates(&self) -> watch::Receiver<()> {
        self.update_tx.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use crate::agent_desired_model::AgentDesiredModel;
    use crate::inference_parameters::InferenceParameters;
    use tokio::time::Duration;
    use tokio::time::timeout;

    use super::*;

    fn make_applicable_state() -> BalancerApplicableState {
        BalancerApplicableState {
            agent_desired_state: AgentDesiredState {
                chat_template_override: None,
                inference_parameters: InferenceParameters::default(),
                model: AgentDesiredModel::LocalToAgent("model.gguf".to_owned()),
                multimodal_projection: AgentDesiredModel::None,
            },
        }
    }

    #[tokio::test]
    async fn set_balancer_applicable_state_wakes_subscribed_waiter() {
        let holder = BalancerApplicableStateHolder::default();
        let mut update_rx = holder.subscribe_to_updates();

        holder.set_balancer_applicable_state(Some(make_applicable_state()));

        timeout(Duration::from_secs(1), update_rx.changed())
            .await
            .expect("waiter did not awaken before the timeout elapsed")
            .expect("watch sender was dropped before the change arrived");
    }

    #[test]
    fn get_balancer_applicable_state_returns_stored_value() {
        let holder = BalancerApplicableStateHolder::default();

        assert!(holder.get_balancer_applicable_state().is_none());

        let applicable_state = make_applicable_state();

        holder.set_balancer_applicable_state(Some(applicable_state.clone()));

        let stored = holder
            .get_balancer_applicable_state()
            .expect("state should be present after set");

        assert_eq!(
            stored.agent_desired_state.model,
            applicable_state.agent_desired_state.model
        );
    }

    #[test]
    fn get_agent_desired_state_returns_none_before_any_state_is_set() {
        let holder = BalancerApplicableStateHolder::default();

        assert!(holder.get_agent_desired_state().is_none());
    }

    #[test]
    fn get_agent_desired_state_returns_stored_agent_desired_state() {
        let holder = BalancerApplicableStateHolder::default();
        let applicable_state = make_applicable_state();

        holder.set_balancer_applicable_state(Some(applicable_state.clone()));

        let stored = holder
            .get_agent_desired_state()
            .expect("agent desired state should be present after set");

        assert_eq!(stored.model, applicable_state.agent_desired_state.model);
    }

    #[test]
    fn set_balancer_applicable_state_can_clear_back_to_none() {
        let holder = BalancerApplicableStateHolder::default();

        holder.set_balancer_applicable_state(Some(make_applicable_state()));
        holder.set_balancer_applicable_state(None);

        assert!(holder.get_balancer_applicable_state().is_none());
        assert!(holder.get_agent_desired_state().is_none());
    }
}
