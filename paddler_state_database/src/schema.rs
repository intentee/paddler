use serde::Deserialize;
use serde::Serialize;

use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_mode::InferenceMode;

use crate::state_database_schema_version::StateDatabaseSchemaVersion;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Schema {
    pub balancer_desired_state: BalancerDesiredState,
    pub version: StateDatabaseSchemaVersion,
}

impl Schema {
    #[must_use]
    pub fn unconfigured(inference_mode: InferenceMode) -> Self {
        Self {
            balancer_desired_state: BalancerDesiredState::unconfigured(inference_mode),
            version: StateDatabaseSchemaVersion::V2,
        }
    }
}
