use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum InferenceMode {
    Decision,
    Embeddings,
    #[default]
    TextGeneration,
}
