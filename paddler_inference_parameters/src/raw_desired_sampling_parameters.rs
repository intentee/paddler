use serde::Deserialize;

use crate::model_default_or::ModelDefaultOr;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawDesiredSamplingParameters {
    #[serde(default)]
    pub min_p: ModelDefaultOr<f32>,
    #[serde(default)]
    pub penalty_frequency: ModelDefaultOr<f32>,
    #[serde(default)]
    pub penalty_last_n: ModelDefaultOr<i32>,
    #[serde(default)]
    pub penalty_presence: ModelDefaultOr<f32>,
    #[serde(default)]
    pub penalty_repeat: ModelDefaultOr<f32>,
    #[serde(default)]
    pub temperature: ModelDefaultOr<f32>,
    #[serde(default)]
    pub top_k: ModelDefaultOr<i32>,
    #[serde(default)]
    pub top_p: ModelDefaultOr<f32>,
}
