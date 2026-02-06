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

    fn make_doc(id: &str, content: &str) -> EmbeddingInputDocument {
        EmbeddingInputDocument {
            content: content.to_string(),
            id: id.to_string(),
        }
    }

    fn make_params(docs: Vec<EmbeddingInputDocument>) -> GenerateEmbeddingBatchParams {
        GenerateEmbeddingBatchParams {
            input_batch: docs,
            normalization_method: EmbeddingNormalizationMethod::None,
        }
    }

    #[test]
    fn test_chunk_by_input_size() {
        let params = make_params(vec![
            make_doc("1", "Hello"),
            make_doc("2", "World"),
            make_doc("3", "This is a test"),
        ]);

        let batches = params.chunk_by_input_size(10).collect::<Vec<_>>();

        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].input_batch.len(), 2);
        assert_eq!(batches[0].input_batch[0].id, "1");
        assert_eq!(batches[0].input_batch[1].id, "2");
        assert_eq!(batches[1].input_batch.len(), 1);
        assert_eq!(batches[1].input_batch[0].id, "3");
    }

    #[test]
    fn test_chunk_empty_batch() {
        let params = make_params(vec![]);
        let batches = params.chunk_by_input_size(100).collect::<Vec<_>>();

        assert!(batches.is_empty());
    }

    #[test]
    fn test_chunk_single_item_larger_than_chunk_size() {
        let params = make_params(vec![make_doc("1", "This content exceeds the chunk limit")]);

        let batches = params.chunk_by_input_size(5).collect::<Vec<_>>();

        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].input_batch.len(), 1);
        assert_eq!(batches[0].input_batch[0].id, "1");
    }

    #[test]
    fn test_chunk_oversized_item_does_not_merge_with_next() {
        let params = make_params(vec![
            make_doc("1", "This is way too long for chunk"),
            make_doc("2", "Short"),
        ]);

        let batches = params.chunk_by_input_size(5).collect::<Vec<_>>();

        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].input_batch[0].id, "1");
        assert_eq!(batches[1].input_batch[0].id, "2");
    }

    #[test]
    fn test_chunk_exact_fit() {
        let params = make_params(vec![make_doc("1", "12345"), make_doc("2", "67890")]);

        // 5 + 5 = 10, exactly the chunk size
        let batches = params.chunk_by_input_size(10).collect::<Vec<_>>();

        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].input_batch.len(), 2);
    }

    #[test]
    fn test_chunk_one_over_limit_splits() {
        let params = make_params(vec![make_doc("1", "12345"), make_doc("2", "678901")]);

        // 5 + 6 = 11, exceeds chunk_size of 10
        let batches = params.chunk_by_input_size(10).collect::<Vec<_>>();

        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].input_batch.len(), 1);
        assert_eq!(batches[1].input_batch.len(), 1);
    }

    #[test]
    fn test_chunk_preserves_normalization_method() {
        let params = GenerateEmbeddingBatchParams {
            input_batch: vec![make_doc("1", "test")],
            normalization_method: EmbeddingNormalizationMethod::L2,
        };

        let batches = params.chunk_by_input_size(100).collect::<Vec<_>>();

        assert!(matches!(
            batches[0].normalization_method,
            EmbeddingNormalizationMethod::L2
        ));
    }

    #[test]
    fn test_chunk_counts_unicode_chars_not_bytes() {
        // "café" is 4 chars but 5 bytes (é is 2 bytes)
        let params = make_params(vec![make_doc("1", "café"), make_doc("2", "naïve")]);

        // 4 chars + 5 chars = 9, fits in chunk_size of 9
        let batches = params.chunk_by_input_size(9).collect::<Vec<_>>();

        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].input_batch.len(), 2);
    }
}
