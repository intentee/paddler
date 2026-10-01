use std::path::Path;

use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModelPath {
    pub model_path: String,
}

impl From<&Path> for ModelPath {
    fn from(path: &Path) -> Self {
        Self {
            model_path: path.display().to_string(),
        }
    }
}
