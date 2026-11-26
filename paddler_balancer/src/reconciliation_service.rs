use std::sync::Arc;

use anyhow::Result;
use tokio::select;
use tokio_util::sync::CancellationToken;
use trzcina::service::Service;

use crate::agent_controller_pool::AgentControllerPool;
use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use crate::balancer_error::BalancerError;

pub struct ReconciliationService {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
}

impl ReconciliationService {
    async fn apply_desired_states_until_shutdown(
        self: Box<Self>,
        shutdown: CancellationToken,
    ) -> Result<(), BalancerError> {
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
                    changed.map_err(BalancerError::ApplicableStatesNoLongerAnnounced)?;

                    let balancer_applicable_state =
                        balancer_applicable_state_rx.borrow_and_update().clone();

                    agent_controller_pool.set_desired_state(&balancer_applicable_state);
                }
            }
        }
    }
}

impl Service for ReconciliationService {
    type Error = BalancerError;

    fn name(&self) -> &'static str {
        "balancer::reconciliation_service"
    }

    async fn run(self: Box<Self>, shutdown: CancellationToken) -> Result<(), BalancerError> {
        self.apply_desired_states_until_shutdown(shutdown).await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tokio::sync::watch;
    use tokio_util::sync::CancellationToken;
    use trzcina::service::Service as _;

    use paddler_messaging::balancer_desired_state::BalancerDesiredState;

    use super::ReconciliationService;
    use crate::agent_controller_pool::AgentControllerPool;
    use crate::balancer_applicable_state::BalancerApplicableState;
    use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
    use crate::balancer_error::BalancerError;

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
        let reconciliation_error = Box::new(reconciliation_service_of_a_closed_store())
            .run(CancellationToken::new())
            .await
            .expect_err("a closed desired state store must fail the reconciliation service");

        assert!(matches!(
            reconciliation_error,
            BalancerError::ApplicableStatesNoLongerAnnounced(_)
        ));
    }
}
