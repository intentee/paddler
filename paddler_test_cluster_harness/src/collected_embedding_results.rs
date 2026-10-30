use std::collections::BTreeSet;

use paddler_messaging::embedding_document_exceeding_sequence_context_details::EmbeddingDocumentExceedingSequenceContextDetails;
use paddler_messaging::embedding_document_tokenized_to_nothing_details::EmbeddingDocumentTokenizedToNothingDetails;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
use paddler_messaging::oversized_embedding_document_details::OversizedEmbeddingDocumentDetails;

use crate::embedding_with_producer::EmbeddingWithProducer;

pub struct CollectedEmbeddingResults {
    pub documents_exceeding_sequence_context: Vec<EmbeddingDocumentExceedingSequenceContextDetails>,
    pub documents_tokenized_to_nothing: Vec<EmbeddingDocumentTokenizedToNothingDetails>,
    pub embeddings: Vec<EmbeddingWithProducer>,
    pub failures: Vec<EmbeddingResult>,
    pub no_embeddings_produced_count: usize,
    pub oversized_documents: Vec<OversizedEmbeddingDocumentDetails>,
    pub request_ids: BTreeSet<String>,
    pub saw_done: bool,
    pub wire_errors: Vec<JsonRpcError>,
}
