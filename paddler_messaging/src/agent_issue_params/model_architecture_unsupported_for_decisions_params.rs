use serde::Deserialize;
use serde::Serialize;

use crate::agent_issue_params::model_path::ModelPath;

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModelArchitectureUnsupportedForDecisionsParams {
    pub architecture: String,
    pub model_path: ModelPath,
}
