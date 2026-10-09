use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum AgentDesiredModel {
    #[default]
    None,
    Uri(String),
}
