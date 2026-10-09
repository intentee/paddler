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
