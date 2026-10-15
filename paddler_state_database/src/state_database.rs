use async_trait::async_trait;

use paddler_messaging::balancer_desired_state::BalancerDesiredState;

use crate::state_database_error::StateDatabaseError;
use crate::stored_balancer_desired_state::StoredBalancerDesiredState;

#[async_trait]
pub trait StateDatabase: Send + Sync {
    async fn read_balancer_desired_state(
        &self,
    ) -> Result<StoredBalancerDesiredState, StateDatabaseError>;

    async fn store_balancer_desired_state(
        &self,
        state: &BalancerDesiredState,
    ) -> Result<(), StateDatabaseError>;
}
