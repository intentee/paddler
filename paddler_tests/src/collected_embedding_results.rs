use paddler_types::jsonrpc::Error as JsonRpcError;
use paddler_types::oversized_embedding_document_details::OversizedEmbeddingDocumentDetails;

use crate::embedding_with_producer::EmbeddingWithProducer;

pub struct CollectedEmbeddingResults {
    pub embeddings: Vec<EmbeddingWithProducer>,
    pub embeddings_disabled: bool,
    pub errors: Vec<String>,
    pub oversized_documents: Vec<OversizedEmbeddingDocumentDetails>,
    pub saw_done: bool,
    pub wire_errors: Vec<JsonRpcError>,
}
