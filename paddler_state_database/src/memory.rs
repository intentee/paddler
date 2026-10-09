use async_trait::async_trait;
use parking_lot::RwLock;
use tokio::sync::watch;

use paddler_messaging::balancer_desired_state::BalancerDesiredState;

use crate::state_database::StateDatabase;
use crate::state_database_error::StateDatabaseError;

pub struct Memory {
    balancer_desired_state: RwLock<BalancerDesiredState>,
    balancer_desired_state_notify_tx: watch::Sender<BalancerDesiredState>,
}

impl Memory {
    #[must_use]
    pub const fn new(
        balancer_desired_state_notify_tx: watch::Sender<BalancerDesiredState>,
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
    async fn read_balancer_desired_state(
        &self,
    ) -> Result<BalancerDesiredState, StateDatabaseError> {
        Ok(self.balancer_desired_state.read().clone())
    }

    async fn store_balancer_desired_state(
        &self,
        state: &BalancerDesiredState,
    ) -> Result<(), StateDatabaseError> {
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

    #[tokio::test]
    async fn reads_back_the_stored_desired_state() {
        let (balancer_desired_state_tx, _balancer_desired_state_rx) =
            watch::channel(BalancerDesiredState::default());
        let database = Memory::new(balancer_desired_state_tx, BalancerDesiredState::default());
        let desired_state = BalancerDesiredState {
            inference_mode: InferenceMode::Embeddings,
            model: AgentDesiredModel::Uri("test_model_path".to_owned()),
            ..BalancerDesiredState::default()
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
}
