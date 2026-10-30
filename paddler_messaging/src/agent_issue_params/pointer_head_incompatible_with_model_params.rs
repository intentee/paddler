use serde::Deserialize;
use serde::Serialize;

use crate::agent_issue_params::model_path::ModelPath;
use crate::agent_issue_params::pointer_head_incompatibility::PointerHeadIncompatibility;

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PointerHeadIncompatibleWithModelParams {
    pub incompatibility: PointerHeadIncompatibility,
    pub pointer_head_path: ModelPath,
}
