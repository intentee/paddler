use serde::Deserialize;
use serde::Serialize;

use paddler_inference_parameters::pooling_type::PoolingType;

use crate::embedding_normalization_method::EmbeddingNormalizationMethod;

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Embedding {
    pub embedding: Vec<f32>,
    pub normalization_method: EmbeddingNormalizationMethod,
    pub pooling_type: PoolingType,
    pub source_document_id: String,
}
