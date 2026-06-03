use serde::Deserialize;
use serde::Serialize;

use crate::agent_issue_params::ModelPath;

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HuggingFaceDownloadLock {
    pub lock_path: String,
    pub model_path: ModelPath,
}
