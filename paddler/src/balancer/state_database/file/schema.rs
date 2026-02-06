use serde::Deserialize;
use serde::Serialize;

use crate::balancer_desired_state::BalancerDesiredState;

fn default_version() -> String {
    "1".into()
}

#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Schema {
    pub balancer_desired_state: BalancerDesiredState,
    #[serde(default = "default_version")]
    pub version: String,
}
