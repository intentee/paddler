use serde::Deserialize;
use serde::Serialize;

use crate::huggingface_model_reference::HuggingFaceModelReference;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum AgentDesiredModel {
    HuggingFace(HuggingFaceModelReference),
    LocalToAgent(String),
    #[default]
    None,
}
