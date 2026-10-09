use std::collections::BTreeSet;

use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
use paddler_messaging::oversized_embedding_document_details::OversizedEmbeddingDocumentDetails;

use crate::embedding_with_producer::EmbeddingWithProducer;

pub struct CollectedEmbeddingResults {
    pub embeddings: Vec<EmbeddingWithProducer>,
    pub failures: Vec<EmbeddingResult>,
    pub model_not_loaded_count: usize,
    pub no_embeddings_produced_count: usize,
    pub oversized_documents: Vec<OversizedEmbeddingDocumentDetails>,
    pub request_ids: BTreeSet<String>,
    pub saw_done: bool,
    pub wire_errors: Vec<JsonRpcError>,
}
