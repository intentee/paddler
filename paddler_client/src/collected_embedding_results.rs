use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
use paddler_messaging::oversized_embedding_document_details::OversizedEmbeddingDocumentDetails;

use crate::embedding_with_producer::EmbeddingWithProducer;

pub struct CollectedEmbeddingResults {
    pub embeddings: Vec<EmbeddingWithProducer>,
    pub embeddings_disabled: bool,
    pub errors: Vec<String>,
    pub embedding_rejected_due_to_active_token_generation_count: usize,
    pub no_embeddings_produced_count: usize,
    pub oversized_documents: Vec<OversizedEmbeddingDocumentDetails>,
    pub saw_done: bool,
    pub wire_errors: Vec<JsonRpcError>,
}
