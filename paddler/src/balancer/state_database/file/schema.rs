use crate::balancer_desired_state::BalancerDesiredState;
use serde::Deserialize;
use serde::Serialize;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_version_is_one() {
        assert_eq!(default_version(), "1");
    }
}
