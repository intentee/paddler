use std::sync::RwLock;

use tokio::sync::watch;

use crate::agent_desired_state::AgentDesiredState;
use crate::balancer_applicable_state::BalancerApplicableState;
use crate::subscribes_to_updates::SubscribesToUpdates;

pub struct BalancerApplicableStateHolder {
    update_tx: watch::Sender<()>,
    balancer_applicable_state: RwLock<Option<BalancerApplicableState>>,
}

impl BalancerApplicableStateHolder {
    #[expect(clippy::expect_used, reason = "mutex lock poison is unrecoverable")]
    pub fn get_agent_desired_state(&self) -> Option<AgentDesiredState> {
        self.balancer_applicable_state
            .read()
            .expect("Failed to get balancer state lock")
            .as_ref()
            .map(|state| state.agent_desired_state.clone())
    }

    #[expect(clippy::expect_used, reason = "mutex lock poison is unrecoverable")]
    pub fn get_balancer_applicable_state(&self) -> Option<BalancerApplicableState> {
        self.balancer_applicable_state
            .read()
            .expect("Failed to get balancer state lock")
            .clone()
    }

    pub fn set_balancer_applicable_state(
        &self,
        balancer_applicable_state: Option<BalancerApplicableState>,
    ) {
        {
            #[expect(clippy::expect_used, reason = "mutex lock poison is unrecoverable")]
            let mut lock = self
                .balancer_applicable_state
                .write()
                .expect("Failed to get balancer state lock");

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
    use anyhow::Result;
    use paddler_types::agent_desired_model::AgentDesiredModel;
    use paddler_types::inference_parameters::InferenceParameters;
    use tokio::time::Duration;
    use tokio::time::timeout;

    use super::*;

    fn make_applicable_state() -> BalancerApplicableState {
        BalancerApplicableState {
            agent_desired_state: AgentDesiredState {
                chat_template_override: None,
                inference_parameters: InferenceParameters::default(),
                model: AgentDesiredModel::None,
                multimodal_projection: AgentDesiredModel::None,
            },
        }
    }

    #[tokio::test]
    async fn set_balancer_applicable_state_wakes_subscribed_waiter() -> Result<()> {
        let holder = BalancerApplicableStateHolder::default();
        let mut update_rx = holder.subscribe_to_updates();

        holder.set_balancer_applicable_state(Some(make_applicable_state()));

        timeout(Duration::from_secs(1), update_rx.changed())
            .await
            .map_err(|error| anyhow::anyhow!("waiter did not awaken: {error}"))?
            .map_err(|error| anyhow::anyhow!("watch sender dropped: {error}"))?;

        Ok(())
    }

    #[test]
    fn get_balancer_applicable_state_returns_stored_value() -> Result<()> {
        let holder = BalancerApplicableStateHolder::default();

        assert!(holder.get_balancer_applicable_state().is_none());

        let applicable_state = make_applicable_state();

        holder.set_balancer_applicable_state(Some(applicable_state.clone()));

        let stored = holder
            .get_balancer_applicable_state()
            .ok_or_else(|| anyhow::anyhow!("state should be present after set"))?;

        assert_eq!(
            stored.agent_desired_state.model,
            applicable_state.agent_desired_state.model
        );

        Ok(())
    }
}
