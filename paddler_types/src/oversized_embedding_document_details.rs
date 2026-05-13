use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OversizedEmbeddingDocumentDetails {
    pub document_tokens: u32,
    pub n_batch: u32,
    pub source_document_id: String,
}
