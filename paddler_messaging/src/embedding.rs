use serde::Deserialize;
use serde::Serialize;

use crate::embedding_normalization_method::EmbeddingNormalizationMethod;
use crate::pooling_type::PoolingType;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Embedding {
    pub embedding: Vec<f32>,
    pub normalization_method: EmbeddingNormalizationMethod,
    pub pooling_type: PoolingType,
    pub source_document_id: String,
}
