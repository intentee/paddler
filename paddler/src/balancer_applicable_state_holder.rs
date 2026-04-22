use std::sync::Arc;
use std::sync::RwLock;

use tokio::sync::Notify;

use crate::agent_desired_state::AgentDesiredState;
use crate::balancer_applicable_state::BalancerApplicableState;

pub struct BalancerApplicableStateHolder {
    pub update_notifier: Arc<Notify>,
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

        self.update_notifier.notify_waiters();
    }
}

impl Default for BalancerApplicableStateHolder {
    fn default() -> Self {
        Self {
            balancer_applicable_state: RwLock::new(None),
            update_notifier: Arc::new(Notify::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use paddler_types::agent_desired_model::AgentDesiredModel;
    use paddler_types::inference_parameters::InferenceParameters;
    use tokio::sync::oneshot;
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
    async fn set_balancer_applicable_state_wakes_waiters() -> Result<()> {
        let holder = Arc::new(BalancerApplicableStateHolder::default());
        let waiter_holder = holder.clone();
        let (ready_tx, ready_rx) = oneshot::channel::<()>();
        let (wake_tx, wake_rx) = oneshot::channel::<()>();

        let waiter_task = tokio::spawn(async move {
            let notified = waiter_holder.update_notifier.notified();
            tokio::pin!(notified);

            notified.as_mut().enable();
            let _ = ready_tx.send(());
            notified.await;
            let _ = wake_tx.send(());
        });

        ready_rx
            .await
            .map_err(|error| anyhow::anyhow!("waiter failed to signal readiness: {error}"))?;

        holder.set_balancer_applicable_state(Some(make_applicable_state()));

        timeout(Duration::from_secs(1), wake_rx)
            .await
            .map_err(|error| anyhow::anyhow!("waiter did not awaken: {error}"))?
            .map_err(|error| anyhow::anyhow!("wake signal dropped: {error}"))?;

        waiter_task
            .await
            .map_err(|error| anyhow::anyhow!("waiter task panicked: {error}"))?;

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
