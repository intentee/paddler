use serde::Deserialize;
use serde::Serialize;

use crate::embedding_result::EmbeddingResult;
use crate::generated_token_result::GeneratedTokenResult;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum Response {
    Embedding(EmbeddingResult),
    GeneratedToken(GeneratedTokenResult),
    Timeout,
    TooManyBufferedRequests,
}

impl From<EmbeddingResult> for Response {
    fn from(result: EmbeddingResult) -> Self {
        Response::Embedding(result)
    }
}

impl From<GeneratedTokenResult> for Response {
    fn from(result: GeneratedTokenResult) -> Self {
        Response::GeneratedToken(result)
    }
}
