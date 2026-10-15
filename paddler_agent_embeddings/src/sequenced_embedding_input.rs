use crate::embedding_input_tokenized::EmbeddingInputTokenized;

pub struct SequencedEmbeddingInput<'inputs> {
    pub input: &'inputs EmbeddingInputTokenized,
    pub sequence_id: i32,
}
