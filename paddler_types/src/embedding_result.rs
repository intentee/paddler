use serde::Deserialize;
use serde::Serialize;

use crate::embedding::Embedding;
use crate::streamable_result::StreamableResult;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum EmbeddingResult {
    Done,
    Embedding(Embedding),
    Error(String),
}

impl StreamableResult for EmbeddingResult {
    fn is_done(&self) -> bool {
        matches!(self, EmbeddingResult::Done | EmbeddingResult::Error(_))
    }
}
