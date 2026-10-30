use crate::embedding_input_tokenized::EmbeddingInputTokenized;

#[derive(Debug, Eq, PartialEq)]
pub struct BatchedEmbeddingInput {
    pub input: EmbeddingInputTokenized,
    pub request_index: usize,
    pub sequence_id: i32,
}
