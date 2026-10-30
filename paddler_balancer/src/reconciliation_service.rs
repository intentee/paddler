use std::sync::Arc;

use anyhow::Context as _;
use anyhow::Result;
use async_trait::async_trait;
use tokio::select;
use tokio_util::sync::CancellationToken;
use trzcina::Service;

use crate::agent_controller_pool::AgentControllerPool;
use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;

pub struct ReconciliationService {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
}

impl ReconciliationService {
    async fn apply_desired_states_until_shutdown(
        self: Box<Self>,
        shutdown: CancellationToken,
    ) -> Result<()> {
        let Self {
            agent_controller_pool,
            balancer_applicable_state_holder,
        } = *self;
        let mut balancer_applicable_state_rx = balancer_applicable_state_holder.subscribe();

        loop {
            select! {
                biased;
                () = shutdown.cancelled() => break Ok(()),
                changed = balancer_applicable_state_rx.changed() => {
                    changed.context("the desired state store stopped announcing applicable states")?;

                    let balancer_applicable_state =
                        balancer_applicable_state_rx.borrow_and_update().clone();

                    agent_controller_pool.set_desired_state(&balancer_applicable_state);
                }
            }
        }
    }
}

#[async_trait]
impl Service for ReconciliationService {
    fn name(&self) -> &'static str {
        "balancer::reconciliation_service"
    }

    async fn run(self: Box<Self>, shutdown: CancellationToken) -> Result<()> {
        self.apply_desired_states_until_shutdown(shutdown).await
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

    fn reconciliation_service_of_a_closed_store() -> ReconciliationService {
        let (balancer_applicable_state_tx, balancer_applicable_state_rx) =
            watch::channel(Arc::new(BalancerApplicableState::initial(
                BalancerDesiredState::default(),
            )));

        drop(balancer_applicable_state_tx);

        ReconciliationService {
            agent_controller_pool: Arc::new(AgentControllerPool::default()),
            balancer_applicable_state_holder: Arc::new(BalancerApplicableStateHolder::new(
                balancer_applicable_state_rx,
            )),
        }
    }

    #[tokio::test]
    async fn stops_cleanly_when_its_desired_state_store_closes_during_shutdown() {
        let shutdown = CancellationToken::new();

        shutdown.cancel();

        Box::new(reconciliation_service_of_a_closed_store())
            .run(shutdown)
            .await
            .expect("a shutdown must stop the reconciliation service cleanly");
    }

    #[tokio::test]
    async fn fails_once_its_desired_state_store_stops_announcing_applicable_states() {
        assert_eq!(
            Box::new(reconciliation_service_of_a_closed_store())
                .run(CancellationToken::new())
                .await
                .map_err(|reconciliation_error| reconciliation_error.to_string()),
            Err("the desired state store stopped announcing applicable states".to_owned())
        );
    }
}
