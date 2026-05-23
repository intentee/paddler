use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use log::error;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use tokio::sync::broadcast;
use tokio::time::Duration;
use tokio::time::MissedTickBehavior;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use trzcina::Service;

use crate::balancer::agent_controller_pool::AgentControllerPool;
use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use crate::converts_to_applicable_state::ConvertsToApplicableState as _;
use crate::sets_desired_state::SetsDesiredState as _;

async fn convert_to_applicable_state(
    balancer_desired_state: &BalancerDesiredState,
    agent_controller_pool: &AgentControllerPool,
    balancer_applicable_state_holder: &BalancerApplicableStateHolder,
    is_converted_to_applicable_state: &mut bool,
) -> Result<()> {
    let balancer_applicable_state = balancer_desired_state.to_applicable_state(()).await?;

    agent_controller_pool
        .set_desired_state(balancer_applicable_state.agent_desired_state.clone())
        .await?;
    balancer_applicable_state_holder
        .set_balancer_applicable_state(Some(balancer_applicable_state));

    *is_converted_to_applicable_state = true;

    Ok(())
}

async fn try_convert_to_applicable_state(
    balancer_desired_state: &BalancerDesiredState,
    agent_controller_pool: &AgentControllerPool,
    balancer_applicable_state_holder: &BalancerApplicableStateHolder,
    is_converted_to_applicable_state: &mut bool,
) {
    if let Err(err) = convert_to_applicable_state(
        balancer_desired_state,
        agent_controller_pool,
        balancer_applicable_state_holder,
        is_converted_to_applicable_state,
    )
    .await
    {
        error!("Failed to convert to applicable state: {err}");
    }
}

pub struct ReconciliationService {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    pub balancer_desired_state: BalancerDesiredState,
    pub balancer_desired_state_rx: broadcast::Receiver<BalancerDesiredState>,
    pub is_converted_to_applicable_state: bool,
}

#[async_trait]
impl Service for ReconciliationService {
    fn name(&self) -> &'static str {
        "balancer::reconciliation_service"
    }

    async fn run(self: Box<Self>, shutdown: CancellationToken) -> Result<()> {
        let Self {
            agent_controller_pool,
            balancer_applicable_state_holder,
            mut balancer_desired_state,
            mut balancer_desired_state_rx,
            mut is_converted_to_applicable_state,
        } = *self;

        let mut ticker = interval(Duration::from_secs(1));

        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                () = shutdown.cancelled() => break Ok(()),
                _ = ticker.tick() => {
                    if !is_converted_to_applicable_state {
                        try_convert_to_applicable_state(
                            &balancer_desired_state,
                            &agent_controller_pool,
                            &balancer_applicable_state_holder,
                            &mut is_converted_to_applicable_state,
                        ).await;
                    }
                },
                received_balancer_desired_state = balancer_desired_state_rx.recv() => {
                    is_converted_to_applicable_state = false;
                    balancer_desired_state = received_balancer_desired_state?;
                    try_convert_to_applicable_state(
                        &balancer_desired_state,
                        &agent_controller_pool,
                        &balancer_applicable_state_holder,
                        &mut is_converted_to_applicable_state,
                    ).await;
                }
            }
        }
    }
}
