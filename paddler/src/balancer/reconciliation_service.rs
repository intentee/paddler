use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use log::error;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use tokio::sync::broadcast;
use tokio::time::Duration;
use tokio::time::MissedTickBehavior;
use tokio::time::interval;

use crate::balancer::agent_controller_pool::AgentControllerPool;
use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use crate::converts_to_applicable_state::ConvertsToApplicableState as _;
use crate::service::Service;
use crate::sets_desired_state::SetsDesiredState as _;

pub struct ReconciliationService {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    pub balancer_desired_state: BalancerDesiredState,
    pub balancer_desired_state_rx: broadcast::Receiver<BalancerDesiredState>,
    pub is_converted_to_applicable_state: bool,
}

impl ReconciliationService {
    pub async fn convert_to_applicable_state(&mut self) -> Result<()> {
        if let Some(balancer_applicable_state) =
            self.balancer_desired_state.to_applicable_state(()).await?
        {
            self.agent_controller_pool
                .set_desired_state(balancer_applicable_state.agent_desired_state.clone())
                .await?;
            self.balancer_applicable_state_holder
                .set_balancer_applicable_state(Some(balancer_applicable_state));
        }

        self.is_converted_to_applicable_state = true;

        Ok(())
    }

    pub async fn try_convert_to_applicable_state(&mut self) {
        if let Err(err) = self.convert_to_applicable_state().await {
            error!("Failed to convert to applicable state: {err}");
        }
    }
}

#[async_trait]
impl Service for ReconciliationService {
    fn name(&self) -> &'static str {
        "balancer::reconciliation_service"
    }

    async fn run(&mut self, mut shutdown: broadcast::Receiver<()>) -> Result<()> {
        let mut ticker = interval(Duration::from_secs(1));

        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                _ = shutdown.recv() => break Ok(()),
                _ = ticker.tick() => {
                    if !self.is_converted_to_applicable_state {
                        self.try_convert_to_applicable_state().await;
                    }
                },
                balancer_desired_state = self.balancer_desired_state_rx.recv() => {
                    self.is_converted_to_applicable_state = false;
                    self.balancer_desired_state = balancer_desired_state?;
                    self.try_convert_to_applicable_state().await;
                }
            }
        }
    }
}
