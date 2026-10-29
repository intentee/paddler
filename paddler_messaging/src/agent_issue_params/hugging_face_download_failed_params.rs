use serde::Deserialize;
use serde::Serialize;

use crate::agent_issue_params::hugging_face_download_failure::HuggingFaceDownloadFailure;
use crate::agent_issue_params::model_path::ModelPath;

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HuggingFaceDownloadFailedParams {
    pub failure: HuggingFaceDownloadFailure,
    pub model_path: ModelPath,
}
