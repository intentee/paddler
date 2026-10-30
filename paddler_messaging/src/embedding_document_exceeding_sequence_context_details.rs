use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EmbeddingDocumentExceedingSequenceContextDetails {
    pub document_tokens: usize,
    pub sequence_context_size: usize,
    pub source_document_id: String,
}
