use anyhow::Result;
use async_trait::async_trait;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use parking_lot::RwLock;
use tokio::sync::broadcast;

use super::StateDatabase;

pub struct Memory {
    balancer_desired_state: RwLock<BalancerDesiredState>,
    balancer_desired_state_notify_tx: broadcast::Sender<BalancerDesiredState>,
}

impl Memory {
    #[must_use]
    pub const fn new(
        balancer_desired_state_notify_tx: broadcast::Sender<BalancerDesiredState>,
        initial_desired_state: BalancerDesiredState,
    ) -> Self {
        Self {
            balancer_desired_state: RwLock::new(initial_desired_state),
            balancer_desired_state_notify_tx,
        }
    }
}

#[async_trait]
impl StateDatabase for Memory {
    async fn read_balancer_desired_state(&self) -> Result<BalancerDesiredState> {
        Ok(self.balancer_desired_state.read().clone())
    }

    async fn store_balancer_desired_state(&self, state: &BalancerDesiredState) -> Result<()> {
        {
            let mut balancer_desired_state = self.balancer_desired_state.write();

            *balancer_desired_state = state.clone();
        }

        self.balancer_desired_state_notify_tx.send(state.clone())?;

        Ok(())
    }
}
