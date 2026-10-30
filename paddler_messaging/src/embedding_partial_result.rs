use crate::embedding::Embedding;
use crate::embedding_document_exceeding_sequence_context_details::EmbeddingDocumentExceedingSequenceContextDetails;
use crate::embedding_document_tokenized_to_nothing_details::EmbeddingDocumentTokenizedToNothingDetails;
use crate::oversized_embedding_document_details::OversizedEmbeddingDocumentDetails;

#[derive(Debug, PartialEq)]
pub enum EmbeddingPartialResult {
    DocumentExceedsBatchSize(OversizedEmbeddingDocumentDetails),
    DocumentExceedsSequenceContext(EmbeddingDocumentExceedingSequenceContextDetails),
    DocumentTokenizedToNothing(EmbeddingDocumentTokenizedToNothingDetails),
    Embedding(Embedding),
}
