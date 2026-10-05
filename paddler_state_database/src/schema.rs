use serde::Deserialize;
use serde::Serialize;

use paddler_messaging::balancer_desired_state::BalancerDesiredState;

use crate::state_database_schema_version::StateDatabaseSchemaVersion;

#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Schema {
    pub balancer_desired_state: BalancerDesiredState,
    pub version: StateDatabaseSchemaVersion,
}

#[cfg(test)]
mod tests {
    use serde_json::from_value;
    use serde_json::json;
    use serde_json::to_value;

    use paddler_messaging::balancer_desired_state::BalancerDesiredState;

    use super::Schema;

    #[test]
    fn rejects_a_schema_without_a_known_version() {
        let balancer_desired_state = to_value(BalancerDesiredState::default()).unwrap();

        assert!(
            from_value::<Schema>(json!({
                "balancer_desired_state": balancer_desired_state,
                "version": "2",
            }))
            .is_err()
        );
    }
}
