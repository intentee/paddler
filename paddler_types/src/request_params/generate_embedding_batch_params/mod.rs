mod chunk_by_input_size_iter;

use serde::Deserialize;
use serde::Serialize;

use self::chunk_by_input_size_iter::ChunkByInputSizeIter;
use crate::embedding_input_document::EmbeddingInputDocument;
use crate::embedding_normalization_method::EmbeddingNormalizationMethod;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GenerateEmbeddingBatchParams {
    pub input_batch: Vec<EmbeddingInputDocument>,
    pub normalization_method: EmbeddingNormalizationMethod,
}

impl GenerateEmbeddingBatchParams {
    /// Input size is the total number of characters in the resulting batches.
    pub fn chunk_by_input_size<'embedding>(
        &'embedding self,
        chunk_size: usize,
    ) -> ChunkByInputSizeIter<'embedding> {
        ChunkByInputSizeIter {
            input_batch: &self.input_batch,
            normalization_method: &self.normalization_method,
            chunk_size,
            current_index: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_by_input_size() {
        let params = GenerateEmbeddingBatchParams {
            input_batch: vec![
                EmbeddingInputDocument {
                    content: "Hello".to_string(),
                    id: "1".to_string(),
                },
                EmbeddingInputDocument {
                    content: "World".to_string(),
                    id: "2".to_string(),
                },
                EmbeddingInputDocument {
                    content: "This is a test".to_string(),
                    id: "3".to_string(),
                },
            ],
            normalization_method: EmbeddingNormalizationMethod::None,
        };

        let batches = params.chunk_by_input_size(10).collect::<Vec<_>>();

        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].input_batch.len(), 2);
        assert_eq!(batches[0].input_batch[0].id, "1");
        assert_eq!(batches[0].input_batch[1].id, "2");
        assert_eq!(batches[1].input_batch.len(), 1);
        assert_eq!(batches[1].input_batch[0].id, "3");
    }
}
