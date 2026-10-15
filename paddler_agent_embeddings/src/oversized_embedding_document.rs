use paddler_messaging::embedding_document_exceeding_sequence_context_details::EmbeddingDocumentExceedingSequenceContextDetails;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::oversized_embedding_document_details::OversizedEmbeddingDocumentDetails;

#[derive(Debug)]
pub enum OversizedEmbeddingDocument {
    ExceedsBatchSize(OversizedEmbeddingDocumentDetails),
    ExceedsSequenceContext(EmbeddingDocumentExceedingSequenceContextDetails),
}

impl OversizedEmbeddingDocument {
    #[must_use]
    pub fn into_embedding_result(self) -> EmbeddingResult {
        match self {
            Self::ExceedsBatchSize(details) => EmbeddingResult::DocumentExceedsBatchSize(details),
            Self::ExceedsSequenceContext(details) => {
                EmbeddingResult::DocumentExceedsSequenceContext(details)
            }
        }
    }
}
