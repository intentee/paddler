use std::mem;

use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum EmbeddingNormalizationMethod {
    L2,
    None,
    RmsNorm { epsilon: f32 },
}

impl EmbeddingNormalizationMethod {
    pub fn can_transform_to(&self, _other: &EmbeddingNormalizationMethod) -> bool {
        if matches!(self, EmbeddingNormalizationMethod::None) {
            return true;
        }

        false
    }

    pub fn needs_transformation_to(&self, other: &EmbeddingNormalizationMethod) -> bool {
        mem::discriminant(self) != mem::discriminant(other)
    }
}
