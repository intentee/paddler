use async_trait::async_trait;
use parking_lot::RwLock;
use tokio::sync::watch;

use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_mode::InferenceMode;

use crate::ensure_requested_state_serves_cluster_mode::ensure_requested_state_serves_cluster_mode;
use crate::ensure_stored_state_serves_cluster_mode::ensure_stored_state_serves_cluster_mode;
use crate::state_database::StateDatabase;
use crate::state_database_error::StateDatabaseError;

pub struct Memory {
    balancer_desired_state: RwLock<BalancerDesiredState>,
    balancer_desired_state_notify_tx: watch::Sender<BalancerDesiredState>,
    cluster_inference_mode: InferenceMode,
}

impl Memory {
    #[must_use]
    pub const fn new(
        balancer_desired_state_notify_tx: watch::Sender<BalancerDesiredState>,
        cluster_inference_mode: InferenceMode,
        initial_desired_state: BalancerDesiredState,
    ) -> Self {
        Self {
            balancer_desired_state: RwLock::new(initial_desired_state),
            balancer_desired_state_notify_tx,
            cluster_inference_mode,
        }
    }
}

#[async_trait]
impl StateDatabase for Memory {
    async fn read_balancer_desired_state(
        &self,
    ) -> Result<BalancerDesiredState, StateDatabaseError> {
        ensure_stored_state_serves_cluster_mode(
            self.cluster_inference_mode,
            self.balancer_desired_state.read().clone(),
        )
    }

    async fn store_balancer_desired_state(
        &self,
        state: &BalancerDesiredState,
    ) -> Result<(), StateDatabaseError> {
        ensure_requested_state_serves_cluster_mode(self.cluster_inference_mode, state)?;

        {
            let mut balancer_desired_state = self.balancer_desired_state.write();

            *balancer_desired_state = state.clone();
        }

        self.balancer_desired_state_notify_tx
            .send_replace(state.clone());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::watch;

    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;
    use paddler_messaging::inference_mode::InferenceMode;

    use super::Memory;
    use crate::state_database::StateDatabase;
    use crate::state_database_error::StateDatabaseError;

    fn embeddings_memory() -> Memory {
        let unconfigured_state = BalancerDesiredState::unconfigured(InferenceMode::Embeddings);
        let (balancer_desired_state_tx, _balancer_desired_state_rx) =
            watch::channel(unconfigured_state.clone());

        Memory::new(
            balancer_desired_state_tx,
            InferenceMode::Embeddings,
            unconfigured_state,
        )
    }

    #[tokio::test]
    async fn reads_back_the_stored_desired_state() {
        let database = embeddings_memory();
        let desired_state = BalancerDesiredState {
            model: AgentDesiredModel::LocalToAgent("test_model_path".to_owned()),
            ..BalancerDesiredState::unconfigured(InferenceMode::Embeddings)
        };

        database
            .store_balancer_desired_state(&desired_state)
            .await
            .unwrap();

        assert_eq!(
            database.read_balancer_desired_state().await.unwrap(),
            desired_state
        );
    }

    #[tokio::test]
    async fn storing_a_state_of_another_mode_is_refused_and_keeps_the_stored_state() {
        let database = embeddings_memory();

        let store_result = database
            .store_balancer_desired_state(&BalancerDesiredState::unconfigured(
                InferenceMode::TextGeneration,
            ))
            .await;

        assert!(matches!(
            store_result,
            Err(StateDatabaseError::RequestedStateServesAnotherMode { .. })
        ));
        assert_eq!(
            database.read_balancer_desired_state().await.unwrap(),
            BalancerDesiredState::unconfigured(InferenceMode::Embeddings)
        );
    }
}
