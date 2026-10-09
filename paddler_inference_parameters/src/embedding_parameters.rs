use std::num::NonZeroUsize;

use serde::Deserialize;
use serde::Serialize;

use crate::pooling_type::PoolingType;

const DEFAULT_EMBEDDING_BATCH_SIZE: NonZeroUsize = NonZeroUsize::new(256).unwrap();

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EmbeddingParameters {
    pub embedding_batch_size: NonZeroUsize,
    pub pooling_type: PoolingType,
}

impl Default for EmbeddingParameters {
    fn default() -> Self {
        Self {
            embedding_batch_size: DEFAULT_EMBEDDING_BATCH_SIZE,
            pooling_type: PoolingType::Last,
        }
    }
}
