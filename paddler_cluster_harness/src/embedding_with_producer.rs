use paddler::embedding::Embedding;

#[derive(Debug)]
pub struct EmbeddingWithProducer {
    pub embedding: Embedding,
    pub generated_by: Option<String>,
}
