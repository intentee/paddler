use super::GenerateEmbeddingBatchParams;
use crate::embedding_input_document::EmbeddingInputDocument;
use crate::embedding_normalization_method::EmbeddingNormalizationMethod;

pub struct ChunkByInputSizeIter<'embedding_batch> {
    pub chunk_size: usize,
    pub current_index: usize,
    pub input_batch: &'embedding_batch [EmbeddingInputDocument],
    pub normalization_method: &'embedding_batch EmbeddingNormalizationMethod,
}

impl<'embedding_batch> Iterator for ChunkByInputSizeIter<'embedding_batch> {
    type Item = GenerateEmbeddingBatchParams;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index >= self.input_batch.len() {
            return None;
        }

        let mut current_batch = Vec::new();
        let mut current_size = 0;

        while self.current_index < self.input_batch.len() {
            let input = &self.input_batch[self.current_index];
            let input_size = input.content.chars().count();

            if current_size + input_size > self.chunk_size && !current_batch.is_empty() {
                break;
            }

            current_batch.push(input.clone());
            current_size += input_size;
            self.current_index += 1;
        }

        if current_batch.is_empty() {
            None
        } else {
            Some(GenerateEmbeddingBatchParams {
                input_batch: current_batch,
                normalization_method: self.normalization_method.clone(),
            })
        }
    }
}
