use std::num::NonZeroUsize;

use serde::Deserialize;

use crate::pooling_type::PoolingType;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawEmbeddingParameters {
    pub embedding_batch_size: NonZeroUsize,
    pub pooling_type: PoolingType,
}
