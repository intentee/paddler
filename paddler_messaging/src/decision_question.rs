use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionQuestion {
    pub id: String,
    pub instructions: String,
    pub options: Vec<String>,
}
