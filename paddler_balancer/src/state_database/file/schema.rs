use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use serde::Deserialize;
use serde::Serialize;

use crate::state_database::file::state_database_schema_version::StateDatabaseSchemaVersion;

#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Schema {
    pub balancer_desired_state: BalancerDesiredState,
    pub version: StateDatabaseSchemaVersion,
}

#[cfg(test)]
mod tests {
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;
    use serde_json::json;

    use super::Schema;

    #[test]
    fn rejects_a_schema_without_a_known_version() {
        let balancer_desired_state = serde_json::to_value(BalancerDesiredState::default()).unwrap();

        assert!(
            serde_json::from_value::<Schema>(json!({
                "balancer_desired_state": balancer_desired_state,
                "version": "2",
            }))
            .is_err()
        );
    }
}
