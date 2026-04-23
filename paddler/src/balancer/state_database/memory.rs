use std::sync::RwLock;

use anyhow::Result;
use async_trait::async_trait;
use paddler_types::balancer_desired_state::BalancerDesiredState;
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
    #[expect(clippy::expect_used, reason = "mutex lock poison is unrecoverable")]
    async fn read_balancer_desired_state(&self) -> Result<BalancerDesiredState> {
        Ok(self
            .balancer_desired_state
            .read()
            .expect("Failed to acquire read lock")
            .clone())
    }

    #[expect(clippy::expect_used, reason = "mutex lock poison is unrecoverable")]
    async fn store_balancer_desired_state(&self, state: &BalancerDesiredState) -> Result<()> {
        {
            let mut balancer_desired_state = self
                .balancer_desired_state
                .write()
                .expect("Failed to acquire write lock");

            *balancer_desired_state = state.clone();
        }

        self.balancer_desired_state_notify_tx.send(state.clone())?;

        Ok(())
    }
}
