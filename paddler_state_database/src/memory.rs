use async_trait::async_trait;
use parking_lot::RwLock;

use paddler_messaging::balancer_desired_state::BalancerDesiredState;

use crate::state_database::StateDatabase;
use crate::state_database_error::StateDatabaseError;
use crate::stored_balancer_desired_state::StoredBalancerDesiredState;

pub struct Memory {
    stored_balancer_desired_state: RwLock<StoredBalancerDesiredState>,
}

impl Memory {
    #[must_use]
    pub const fn new(stored_balancer_desired_state: StoredBalancerDesiredState) -> Self {
        Self {
            stored_balancer_desired_state: RwLock::new(stored_balancer_desired_state),
        }
    }
}

#[async_trait]
impl StateDatabase for Memory {
    async fn read_balancer_desired_state(
        &self,
    ) -> Result<StoredBalancerDesiredState, StateDatabaseError> {
        Ok(self.stored_balancer_desired_state.read().clone())
    }

    async fn store_balancer_desired_state(
        &self,
        state: &BalancerDesiredState,
    ) -> Result<(), StateDatabaseError> {
        *self.stored_balancer_desired_state.write() =
            StoredBalancerDesiredState::Stored(state.clone());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;
    use paddler_messaging::inference_mode::InferenceMode;

    use super::Memory;
    use crate::state_database::StateDatabase;
    use crate::stored_balancer_desired_state::StoredBalancerDesiredState;

    #[tokio::test]
    async fn reads_back_the_stored_desired_state() {
        let database = Memory::new(StoredBalancerDesiredState::NothingStored);
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
            StoredBalancerDesiredState::Stored(desired_state)
        );
    }
}
