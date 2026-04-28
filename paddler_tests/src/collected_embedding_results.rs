use paddler_types::embedding::Embedding;

pub struct CollectedEmbeddingResults {
    pub embeddings: Vec<Embedding>,
    pub errors: Vec<String>,
    pub saw_done: bool,
}
