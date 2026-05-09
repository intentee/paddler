use crate::embedding_with_producer::EmbeddingWithProducer;

pub struct CollectedEmbeddingResults {
    pub embeddings: Vec<EmbeddingWithProducer>,
    pub errors: Vec<String>,
    pub saw_done: bool,
}
