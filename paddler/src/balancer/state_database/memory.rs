use std::sync::RwLock;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::broadcast;

use super::StateDatabase;
use crate::balancer_desired_state::BalancerDesiredState;

pub struct Memory {
    balancer_desired_state: RwLock<BalancerDesiredState>,
    balancer_desired_state_notify_tx: broadcast::Sender<BalancerDesiredState>,
}

impl Memory {
    pub fn new(balancer_desired_state_notify_tx: broadcast::Sender<BalancerDesiredState>) -> Self {
        Memory {
            balancer_desired_state: RwLock::new(BalancerDesiredState::default()),
            balancer_desired_state_notify_tx,
        }
    }
}

#[async_trait]
impl StateDatabase for Memory {
    async fn read_balancer_desired_state(&self) -> Result<BalancerDesiredState> {
        Ok(self
            .balancer_desired_state
            .read()
            .expect("Failed to acquire read lock")
            .clone())
    }

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
