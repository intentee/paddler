use paddler_messaging::embedding_document_exceeding_sequence_context_details::EmbeddingDocumentExceedingSequenceContextDetails;
use paddler_messaging::embedding_document_tokenized_to_nothing_details::EmbeddingDocumentTokenizedToNothingDetails;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::oversized_embedding_document_details::OversizedEmbeddingDocumentDetails;

#[derive(Debug)]
pub enum UnembeddableEmbeddingDocument {
    ExceedsBatchSize(OversizedEmbeddingDocumentDetails),
    ExceedsSequenceContext(EmbeddingDocumentExceedingSequenceContextDetails),
    TokenizedToNothing(EmbeddingDocumentTokenizedToNothingDetails),
}

impl UnembeddableEmbeddingDocument {
    #[must_use]
    pub fn into_embedding_result(self) -> EmbeddingResult {
        match self {
            Self::ExceedsBatchSize(details) => EmbeddingResult::DocumentExceedsBatchSize(details),
            Self::ExceedsSequenceContext(details) => {
                EmbeddingResult::DocumentExceedsSequenceContext(details)
            }
            Self::TokenizedToNothing(details) => {
                EmbeddingResult::DocumentTokenizedToNothing(details)
            }
        }
    }
}
