use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum EmbeddingNormalizationMethod {
    L2,
    None,
    RmsNorm { epsilon: f32 },
}
