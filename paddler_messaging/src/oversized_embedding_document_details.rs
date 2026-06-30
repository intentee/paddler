use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OversizedEmbeddingDocumentDetails {
    pub document_tokens: usize,
    pub n_batch: usize,
    pub source_document_id: String,
}
