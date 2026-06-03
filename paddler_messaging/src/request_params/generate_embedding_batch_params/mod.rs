mod chunk_evenly_with_cap_error;

use serde::Deserialize;
use serde::Serialize;

pub use self::chunk_evenly_with_cap_error::ChunkEvenlyWithCapError;
use crate::embedding_input_document::EmbeddingInputDocument;
use crate::embedding_normalization_method::EmbeddingNormalizationMethod;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GenerateEmbeddingBatchParams {
    pub input_batch: Vec<EmbeddingInputDocument>,
    pub normalization_method: EmbeddingNormalizationMethod,
}

impl GenerateEmbeddingBatchParams {
    pub fn chunk_evenly_with_cap(
        &self,
        agent_count: usize,
        max_documents_per_chunk: usize,
    ) -> Result<Vec<Self>, ChunkEvenlyWithCapError> {
        if agent_count == 0 {
            return Err(ChunkEvenlyWithCapError::ZeroAgentCount);
        }
        if max_documents_per_chunk == 0 {
            return Err(ChunkEvenlyWithCapError::ZeroMaxDocumentsPerChunk);
        }

        let document_count = self.input_batch.len();

        if document_count == 0 {
            return Ok(Vec::new());
        }

        let chunks_to_honor_cap = document_count.div_ceil(max_documents_per_chunk);
        let chunk_count = document_count.min(agent_count.max(chunks_to_honor_cap));

        let quotient = document_count / chunk_count;
        let remainder = document_count % chunk_count;

        let mut sub_batches = Vec::with_capacity(chunk_count);
        let mut start_index = 0;

        for chunk_index in 0..chunk_count {
            let chunk_size = if chunk_index < remainder {
                quotient + 1
            } else {
                quotient
            };

            let end_index = start_index + chunk_size;

            sub_batches.push(Self {
                input_batch: self.input_batch[start_index..end_index].to_vec(),
                normalization_method: self.normalization_method.clone(),
            });

            start_index = end_index;
        }

        Ok(sub_batches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_doc(id: &str, content: &str) -> EmbeddingInputDocument {
        EmbeddingInputDocument {
            content: content.to_owned(),
            id: id.to_owned(),
        }
    }

    fn make_params(docs: Vec<EmbeddingInputDocument>) -> GenerateEmbeddingBatchParams {
        GenerateEmbeddingBatchParams {
            input_batch: docs,
            normalization_method: EmbeddingNormalizationMethod::None,
        }
    }

    fn make_docs(count: usize) -> Vec<EmbeddingInputDocument> {
        (0..count)
            .map(|index| make_doc(&format!("doc{index:05}"), "x"))
            .collect()
    }

    #[test]
    fn chunk_evenly_with_cap_empty_input() {
        let params = make_params(vec![]);

        let sub_batches = params.chunk_evenly_with_cap(4, 256).unwrap();

        assert!(sub_batches.is_empty());
    }

    #[test]
    fn chunk_evenly_with_cap_single_doc_single_agent() {
        let params = make_params(vec![make_doc("only", "content")]);

        let sub_batches = params.chunk_evenly_with_cap(1, 256).unwrap();

        assert_eq!(sub_batches.len(), 1);
        assert_eq!(sub_batches[0].input_batch.len(), 1);
        assert_eq!(sub_batches[0].input_batch[0].id, "only");
    }

    #[test]
    fn chunk_evenly_with_cap_single_doc_many_agents() {
        let params = make_params(vec![make_doc("only", "content")]);

        let sub_batches = params.chunk_evenly_with_cap(5, 256).unwrap();

        assert_eq!(sub_batches.len(), 1);
        assert_eq!(sub_batches[0].input_batch.len(), 1);
        assert_eq!(sub_batches[0].input_batch[0].id, "only");
    }

    #[test]
    fn chunk_evenly_with_cap_more_agents_than_docs_uses_n_chunks() {
        let params = make_params(make_docs(3));

        let sub_batches = params.chunk_evenly_with_cap(5, 256).unwrap();

        assert_eq!(sub_batches.len(), 3);
        for sub_batch in &sub_batches {
            assert_eq!(sub_batch.input_batch.len(), 1);
        }
    }

    #[test]
    fn chunk_evenly_with_cap_rejects_zero_agent_count() {
        let params = make_params(make_docs(5));

        let is_zero_agent_count =
            |result: Result<Vec<GenerateEmbeddingBatchParams>, ChunkEvenlyWithCapError>| {
                matches!(result, Err(ChunkEvenlyWithCapError::ZeroAgentCount))
            };

        assert!(is_zero_agent_count(params.chunk_evenly_with_cap(0, 256)));
        assert!(!is_zero_agent_count(params.chunk_evenly_with_cap(2, 0)));
    }

    #[test]
    fn chunk_evenly_with_cap_rejects_zero_max_documents_per_chunk() {
        let params = make_params(make_docs(4));

        let is_zero_max_documents =
            |result: Result<Vec<GenerateEmbeddingBatchParams>, ChunkEvenlyWithCapError>| {
                matches!(
                    result,
                    Err(ChunkEvenlyWithCapError::ZeroMaxDocumentsPerChunk)
                )
            };

        assert!(is_zero_max_documents(params.chunk_evenly_with_cap(2, 0)));
        assert!(!is_zero_max_documents(params.chunk_evenly_with_cap(0, 0)));
    }

    #[test]
    fn chunk_evenly_with_cap_below_cap_splits_per_agent() {
        let params = make_params(make_docs(4));

        let sub_batches = params.chunk_evenly_with_cap(4, 256).unwrap();

        assert_eq!(sub_batches.len(), 4);
        for sub_batch in &sub_batches {
            assert_eq!(sub_batch.input_batch.len(), 1);
        }
    }

    #[test]
    fn chunk_evenly_with_cap_below_cap_uneven_split() {
        let params = make_params(make_docs(11));

        let sub_batches = params.chunk_evenly_with_cap(4, 256).unwrap();

        assert_eq!(sub_batches.len(), 4);
        assert_eq!(sub_batches[0].input_batch.len(), 3);
        assert_eq!(sub_batches[1].input_batch.len(), 3);
        assert_eq!(sub_batches[2].input_batch.len(), 3);
        assert_eq!(sub_batches[3].input_batch.len(), 2);
    }

    #[test]
    fn chunk_evenly_with_cap_user_example_80_docs_4_agents_cap_100() {
        let params = make_params(make_docs(80));

        let sub_batches = params.chunk_evenly_with_cap(4, 100).unwrap();

        assert_eq!(sub_batches.len(), 4);
        for sub_batch in &sub_batches {
            assert_eq!(sub_batch.input_batch.len(), 20);
        }
    }

    #[test]
    fn chunk_evenly_with_cap_user_example_1000_docs_4_agents_cap_100() {
        let params = make_params(make_docs(1000));

        let sub_batches = params.chunk_evenly_with_cap(4, 100).unwrap();

        assert_eq!(sub_batches.len(), 10);
        for sub_batch in &sub_batches {
            assert_eq!(sub_batch.input_batch.len(), 100);
        }
    }

    #[test]
    fn chunk_evenly_with_cap_at_cap_boundary_uses_agent_count() {
        let params = make_params(make_docs(1024));

        let sub_batches = params.chunk_evenly_with_cap(4, 256).unwrap();

        assert_eq!(sub_batches.len(), 4);
        for sub_batch in &sub_batches {
            assert_eq!(sub_batch.input_batch.len(), 256);
        }
    }

    #[test]
    fn chunk_evenly_with_cap_above_cap_boundary_creates_extra_chunks() {
        let params = make_params(make_docs(2000));

        let sub_batches = params.chunk_evenly_with_cap(4, 256).unwrap();

        assert_eq!(sub_batches.len(), 8);
        for sub_batch in &sub_batches {
            assert_eq!(sub_batch.input_batch.len(), 250);
        }
    }

    #[test]
    fn chunk_evenly_with_cap_far_above_cap_distributes_evenly() {
        let params = make_params(make_docs(1100));

        let sub_batches = params.chunk_evenly_with_cap(4, 256).unwrap();

        assert_eq!(sub_batches.len(), 5);
        for sub_batch in &sub_batches {
            assert_eq!(sub_batch.input_batch.len(), 220);
        }
    }

    #[test]
    fn chunk_evenly_with_cap_extreme_large_n_small_cap() {
        let params = make_params(make_docs(10_000));

        let sub_batches = params.chunk_evenly_with_cap(4, 1).unwrap();

        assert_eq!(sub_batches.len(), 10_000);
        for sub_batch in &sub_batches {
            assert_eq!(sub_batch.input_batch.len(), 1);
        }
    }

    #[test]
    fn chunk_evenly_with_cap_extreme_one_doc_per_chunk() {
        let params = make_params(make_docs(100));

        let sub_batches = params.chunk_evenly_with_cap(100, 256).unwrap();

        assert_eq!(sub_batches.len(), 100);
        for sub_batch in &sub_batches {
            assert_eq!(sub_batch.input_batch.len(), 1);
        }
    }

    #[test]
    fn chunk_evenly_with_cap_no_sub_batch_exceeds_cap_sweep() {
        let document_counts: Vec<usize> = (0..=50).chain([256, 257, 1000, 2001]).collect();
        let agent_counts: Vec<usize> = (1..=8).collect();
        let caps: Vec<usize> = vec![1, 2, 4, 100, 256];

        for &document_count in &document_counts {
            for &agent_count in &agent_counts {
                for &cap in &caps {
                    let params = make_params(make_docs(document_count));

                    let sub_batches = params.chunk_evenly_with_cap(agent_count, cap).unwrap();

                    let total_documents: usize =
                        sub_batches.iter().map(|sub| sub.input_batch.len()).sum();
                    assert_eq!(
                        total_documents, document_count,
                        "total documents must equal N (N={document_count}, agents={agent_count}, cap={cap})",
                    );

                    let largest_sub_batch_size = sub_batches
                        .iter()
                        .map(|sub| sub.input_batch.len())
                        .max()
                        .unwrap_or_default();
                    assert!(
                        largest_sub_batch_size <= cap,
                        "largest sub-batch size {largest_sub_batch_size} exceeds cap {cap} (N={document_count}, agents={agent_count})",
                    );

                    let collected_ids: Vec<String> = sub_batches
                        .iter()
                        .flat_map(|sub| sub.input_batch.iter().map(|doc| doc.id.clone()))
                        .collect();
                    let expected_ids: Vec<String> = (0..document_count)
                        .map(|index| format!("doc{index:05}"))
                        .collect();
                    assert_eq!(
                        collected_ids, expected_ids,
                        "concatenated IDs must equal original order (N={document_count}, agents={agent_count}, cap={cap})",
                    );

                    if document_count > 0 {
                        assert!(
                            !sub_batches.is_empty(),
                            "non-empty input must produce at least one sub-batch (N={document_count}, agents={agent_count}, cap={cap})",
                        );
                        for sub_batch in &sub_batches {
                            assert!(
                                !sub_batch.input_batch.is_empty(),
                                "no sub-batch may be empty (N={document_count}, agents={agent_count}, cap={cap})",
                            );
                        }
                    } else {
                        assert!(sub_batches.is_empty(), "empty input must produce empty Vec");
                    }
                }
            }
        }
    }

    #[test]
    fn chunk_evenly_with_cap_preserves_normalization_method() {
        let params = GenerateEmbeddingBatchParams {
            input_batch: make_docs(8),
            normalization_method: EmbeddingNormalizationMethod::L2,
        };

        let sub_batches = params.chunk_evenly_with_cap(4, 256).unwrap();

        let is_l2 = |normalization_method: &EmbeddingNormalizationMethod| {
            matches!(normalization_method, EmbeddingNormalizationMethod::L2)
        };

        assert_eq!(sub_batches.len(), 4);
        assert!(
            sub_batches
                .iter()
                .all(|sub_batch| is_l2(&sub_batch.normalization_method))
        );
        assert!(!is_l2(&EmbeddingNormalizationMethod::None));
    }

    #[test]
    fn chunk_evenly_with_cap_preserves_document_ids_and_order() {
        let params = make_params(make_docs(12));

        let sub_batches = params.chunk_evenly_with_cap(5, 256).unwrap();

        let collected_ids: Vec<String> = sub_batches
            .iter()
            .flat_map(|sub| sub.input_batch.iter().map(|doc| doc.id.clone()))
            .collect();
        let expected_ids: Vec<String> = (0..12).map(|index| format!("doc{index:05}")).collect();

        assert_eq!(collected_ids, expected_ids);
    }
}
