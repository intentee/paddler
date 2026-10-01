use std::sync::Arc;

use anyhow::Context as _;
use anyhow::Result;
use async_trait::async_trait;
use tokio::select;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use trzcina::Service;

use paddler_messaging::balancer_desired_state::BalancerDesiredState;

use crate::agent_controller_pool::AgentControllerPool;
use crate::balancer_applicable_state::BalancerApplicableState;
use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;

pub struct ReconciliationService {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    pub balancer_desired_state_rx: watch::Receiver<BalancerDesiredState>,
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
            mut balancer_desired_state_rx,
        } = *self;

        loop {
            select! {
                biased;
                () = shutdown.cancelled() => break Ok(()),
                changed = balancer_desired_state_rx.changed() => {
                    changed.context("the state database stopped announcing desired states")?;

                    let balancer_applicable_state = BalancerApplicableState::from(
                        balancer_desired_state_rx.borrow_and_update().clone(),
                    );

                    let agent_desired_state = balancer_applicable_state.agent_desired_state.clone();

                    balancer_applicable_state_holder
                        .set_balancer_applicable_state(balancer_applicable_state);
                    agent_controller_pool.set_desired_state(&agent_desired_state);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tokio::sync::watch;
    use tokio_util::sync::CancellationToken;
    use trzcina::Service as _;

    use paddler_messaging::balancer_desired_state::BalancerDesiredState;

    use super::ReconciliationService;
    use crate::agent_controller_pool::AgentControllerPool;
    use crate::balancer_applicable_state::BalancerApplicableState;
    use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;

    const RUNS_TO_EXERCISE_EVERY_BRANCH_ORDER: usize = 64;

    #[tokio::test]
    async fn stops_cleanly_when_its_state_database_closes_during_shutdown() {
        for _ in 0..RUNS_TO_EXERCISE_EVERY_BRANCH_ORDER {
            let (balancer_desired_state_tx, balancer_desired_state_rx) =
                watch::channel(BalancerDesiredState::default());
            let shutdown = CancellationToken::new();

            shutdown.cancel();
            drop(balancer_desired_state_tx);

            Box::new(ReconciliationService {
                agent_controller_pool: Arc::new(AgentControllerPool::default()),
                balancer_applicable_state_holder: Arc::new(BalancerApplicableStateHolder::new(
                    BalancerApplicableState::from(BalancerDesiredState::default()),
                )),
                balancer_desired_state_rx,
            })
            .run(shutdown)
            .await
            .expect("a shutdown must stop the reconciliation service cleanly");
        }
    }
}
