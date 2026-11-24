use serde::Deserialize;
use serde::Serialize;

use crate::state_database_schema_version::StateDatabaseSchemaVersion;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Schema<TBalancerDesiredState> {
    pub balancer_desired_state: TBalancerDesiredState,
    pub version: StateDatabaseSchemaVersion,
}
