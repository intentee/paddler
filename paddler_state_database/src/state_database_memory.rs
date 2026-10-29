use parking_lot::RwLock;

use paddler_messaging::balancer_desired_state::BalancerDesiredState;

use crate::stored_balancer_desired_state::StoredBalancerDesiredState;

pub struct StateDatabaseMemory {
    stored_balancer_desired_state: RwLock<StoredBalancerDesiredState>,
}

impl StateDatabaseMemory {
    #[must_use]
    pub const fn new(stored_balancer_desired_state: StoredBalancerDesiredState) -> Self {
        Self {
            stored_balancer_desired_state: RwLock::new(stored_balancer_desired_state),
        }
    }

    #[must_use]
    pub fn read_balancer_desired_state(&self) -> StoredBalancerDesiredState {
        self.stored_balancer_desired_state.read().clone()
    }

    pub fn store_balancer_desired_state(&self, balancer_desired_state: &BalancerDesiredState) {
        *self.stored_balancer_desired_state.write() =
            StoredBalancerDesiredState::Stored(Box::new(balancer_desired_state.clone()));
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;
    use paddler_messaging::decision_settings::DecisionSettings;

    use super::StateDatabaseMemory;
    use crate::stored_balancer_desired_state::StoredBalancerDesiredState;

    #[test]
    fn reads_back_the_stored_desired_state() {
        let database = StateDatabaseMemory::new(StoredBalancerDesiredState::NothingStored);
        let desired_state = BalancerDesiredState {
            decision: DecisionSettings {
                pointer_head: AgentDesiredModel::Uri("test_pointer_head_path".to_owned()),
            },
            model: AgentDesiredModel::Uri("test_model_path".to_owned()),
            ..BalancerDesiredState::default()
        };

        database.store_balancer_desired_state(&desired_state);

        assert_eq!(
            database.read_balancer_desired_state(),
            StoredBalancerDesiredState::Stored(Box::new(desired_state))
        );
    }
}
