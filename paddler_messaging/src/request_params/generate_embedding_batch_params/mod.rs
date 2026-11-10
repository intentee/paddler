use std::num::NonZeroUsize;

use serde::Deserialize;
use serde::Serialize;

use crate::embedding_input_document::EmbeddingInputDocument;
use crate::embedding_normalization_method::EmbeddingNormalizationMethod;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GenerateEmbeddingBatchParams {
    pub input_batch: Vec<EmbeddingInputDocument>,
    pub normalization_method: EmbeddingNormalizationMethod,
}

impl GenerateEmbeddingBatchParams {
    #[must_use]
    pub fn chunk_evenly_with_cap(
        self,
        agent_count: usize,
        max_documents_per_chunk: NonZeroUsize,
    ) -> Vec<Self> {
        let Self {
            input_batch,
            normalization_method,
        } = self;
        let Some(document_count) = NonZeroUsize::new(input_batch.len()) else {
            return Vec::new();
        };

        let chunks_to_honor_cap = document_count.div_ceil(max_documents_per_chunk);
        let chunk_count = NonZeroUsize::new(agent_count)
            .map_or(chunks_to_honor_cap, |available_agents| {
                available_agents.max(chunks_to_honor_cap)
            })
            .min(document_count);

        let larger_chunk_size = document_count.div_ceil(chunk_count).get();
        let smaller_chunk_size = document_count.get() / chunk_count;
        let larger_chunk_count = document_count.get() % chunk_count;

        let mut documents = input_batch.into_iter();

        (0..chunk_count.get())
            .map(|chunk_index| {
                let chunk_size = if chunk_index < larger_chunk_count {
                    larger_chunk_size
                } else {
                    smaller_chunk_size
                };

                Self {
                    input_batch: documents.by_ref().take(chunk_size).collect(),
                    normalization_method: normalization_method.clone(),
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use super::GenerateEmbeddingBatchParams;
    use crate::embedding_input_document::EmbeddingInputDocument;
    use crate::embedding_normalization_method::EmbeddingNormalizationMethod;

    fn chunk_cap(max_documents_per_chunk: usize) -> NonZeroUsize {
        NonZeroUsize::new(max_documents_per_chunk).expect("a chunk cap must be non-zero")
    }

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

    fn input_batches(
        sub_batches: Vec<GenerateEmbeddingBatchParams>,
    ) -> Vec<Vec<EmbeddingInputDocument>> {
        sub_batches
            .into_iter()
            .map(|GenerateEmbeddingBatchParams { input_batch, .. }| input_batch)
            .collect()
    }

    fn sub_batch_sizes(sub_batches: &[GenerateEmbeddingBatchParams]) -> Vec<usize> {
        sub_batches
            .iter()
            .map(|GenerateEmbeddingBatchParams { input_batch, .. }| input_batch.len())
            .collect()
    }

    fn chunking_sweep_cases() -> impl Iterator<Item = ChunkingSweepCase> {
        (0..=50)
            .chain([256, 257, 1000, 2001])
            .flat_map(|document_count| {
                (0..=8).flat_map(move |agent_count| {
                    [1, 2, 4, 100, 256].map(move |cap| ChunkingSweepCase {
                        agent_count,
                        cap,
                        document_count,
                        sub_batches: make_params(make_docs(document_count))
                            .chunk_evenly_with_cap(agent_count, chunk_cap(cap)),
                    })
                })
            })
    }

    struct ChunkingSweepCase {
        agent_count: usize,
        cap: usize,
        document_count: usize,
        sub_batches: Vec<GenerateEmbeddingBatchParams>,
    }

    #[test]
    fn chunk_evenly_with_cap_produces_no_sub_batches_from_an_empty_batch() {
        let params = make_params(vec![]);

        let sub_batches = params.chunk_evenly_with_cap(4, chunk_cap(256));

        assert!(sub_batches.is_empty());
    }

    #[test]
    fn chunk_evenly_with_cap_keeps_a_single_document_in_one_sub_batch_for_one_agent() {
        let params = make_params(vec![make_doc("only", "content")]);

        let sub_batches = params.chunk_evenly_with_cap(1, chunk_cap(256));

        assert_eq!(
            input_batches(sub_batches),
            vec![vec![make_doc("only", "content")]]
        );
    }

    #[test]
    fn chunk_evenly_with_cap_keeps_a_single_document_in_one_sub_batch_for_many_agents() {
        let params = make_params(vec![make_doc("only", "content")]);

        let sub_batches = params.chunk_evenly_with_cap(5, chunk_cap(256));

        assert_eq!(
            input_batches(sub_batches),
            vec![vec![make_doc("only", "content")]]
        );
    }

    #[test]
    fn chunk_evenly_with_cap_more_agents_than_docs_uses_n_chunks() {
        let params = make_params(make_docs(3));

        let sub_batches = params.chunk_evenly_with_cap(5, chunk_cap(256));

        assert_eq!(sub_batches.len(), 3);
        for sub_batch in &sub_batches {
            assert_eq!(sub_batch.input_batch.len(), 1);
        }
    }

    #[test]
    fn chunk_evenly_with_cap_below_cap_splits_per_agent() {
        let params = make_params(make_docs(4));

        let sub_batches = params.chunk_evenly_with_cap(4, chunk_cap(256));

        assert_eq!(sub_batches.len(), 4);
        for sub_batch in &sub_batches {
            assert_eq!(sub_batch.input_batch.len(), 1);
        }
    }

    #[test]
    fn chunk_evenly_with_cap_gives_the_remainder_of_an_uneven_split_to_the_first_sub_batches() {
        let params = make_params(make_docs(11));

        let sub_batches = params.chunk_evenly_with_cap(4, chunk_cap(256));

        assert_eq!(sub_batch_sizes(&sub_batches), vec![3, 3, 3, 2]);
    }

    #[test]
    fn chunk_evenly_with_cap_splits_a_batch_below_the_cap_across_every_agent() {
        let params = make_params(make_docs(80));

        let sub_batches = params.chunk_evenly_with_cap(4, chunk_cap(100));

        assert_eq!(sub_batches.len(), 4);
        for sub_batch in &sub_batches {
            assert_eq!(sub_batch.input_batch.len(), 20);
        }
    }

    #[test]
    fn chunk_evenly_with_cap_splits_a_batch_above_the_cap_into_capped_chunks() {
        let params = make_params(make_docs(1000));

        let sub_batches = params.chunk_evenly_with_cap(4, chunk_cap(100));

        assert_eq!(sub_batches.len(), 10);
        for sub_batch in &sub_batches {
            assert_eq!(sub_batch.input_batch.len(), 100);
        }
    }

    #[test]
    fn chunk_evenly_with_cap_at_cap_boundary_uses_agent_count() {
        let params = make_params(make_docs(1024));

        let sub_batches = params.chunk_evenly_with_cap(4, chunk_cap(256));

        assert_eq!(sub_batches.len(), 4);
        for sub_batch in &sub_batches {
            assert_eq!(sub_batch.input_batch.len(), 256);
        }
    }

    #[test]
    fn chunk_evenly_with_cap_above_cap_boundary_creates_extra_chunks() {
        let params = make_params(make_docs(2000));

        let sub_batches = params.chunk_evenly_with_cap(4, chunk_cap(256));

        assert_eq!(sub_batches.len(), 8);
        for sub_batch in &sub_batches {
            assert_eq!(sub_batch.input_batch.len(), 250);
        }
    }

    #[test]
    fn chunk_evenly_with_cap_far_above_cap_distributes_evenly() {
        let params = make_params(make_docs(1100));

        let sub_batches = params.chunk_evenly_with_cap(4, chunk_cap(256));

        assert_eq!(sub_batches.len(), 5);
        for sub_batch in &sub_batches {
            assert_eq!(sub_batch.input_batch.len(), 220);
        }
    }

    #[test]
    fn chunk_evenly_with_cap_of_one_puts_every_document_of_a_large_batch_in_its_own_sub_batch() {
        let params = make_params(make_docs(10_000));

        let sub_batches = params.chunk_evenly_with_cap(4, chunk_cap(1));

        assert_eq!(sub_batches.len(), 10_000);
        for sub_batch in &sub_batches {
            assert_eq!(sub_batch.input_batch.len(), 1);
        }
    }

    #[test]
    fn chunk_evenly_with_cap_gives_each_document_its_own_sub_batch_for_as_many_agents() {
        let params = make_params(make_docs(100));

        let sub_batches = params.chunk_evenly_with_cap(100, chunk_cap(256));

        assert_eq!(sub_batches.len(), 100);
        for sub_batch in &sub_batches {
            assert_eq!(sub_batch.input_batch.len(), 1);
        }
    }

    #[test]
    fn chunk_evenly_with_cap_never_exceeds_the_cap_across_the_sweep() {
        for ChunkingSweepCase {
            agent_count,
            cap,
            document_count,
            sub_batches,
        } in chunking_sweep_cases()
        {
            let largest_sub_batch_size = sub_batches
                .iter()
                .map(|GenerateEmbeddingBatchParams { input_batch, .. }| input_batch.len())
                .max()
                .unwrap_or_default();

            assert!(
                largest_sub_batch_size <= cap,
                "largest sub-batch size {largest_sub_batch_size} exceeds cap {cap} (N={document_count}, agents={agent_count})",
            );
        }
    }

    #[test]
    fn chunk_evenly_with_cap_preserves_document_ids_and_order_across_the_sweep() {
        for ChunkingSweepCase {
            agent_count,
            cap,
            document_count,
            sub_batches,
        } in chunking_sweep_cases()
        {
            let collected_ids: Vec<String> = sub_batches
                .iter()
                .flat_map(|GenerateEmbeddingBatchParams { input_batch, .. }| {
                    input_batch
                        .iter()
                        .map(|EmbeddingInputDocument { id, .. }| id.clone())
                })
                .collect();
            let expected_ids: Vec<String> = (0..document_count)
                .map(|index| format!("doc{index:05}"))
                .collect();

            assert_eq!(
                collected_ids, expected_ids,
                "concatenated IDs must equal original order (N={document_count}, agents={agent_count}, cap={cap})",
            );
        }
    }

    #[test]
    fn chunk_evenly_with_cap_never_produces_an_empty_sub_batch_across_the_sweep() {
        for ChunkingSweepCase {
            agent_count,
            cap,
            document_count,
            sub_batches,
        } in chunking_sweep_cases()
        {
            assert!(
                sub_batches
                    .iter()
                    .all(
                        |GenerateEmbeddingBatchParams { input_batch, .. }| !input_batch.is_empty()
                    ),
                "no sub-batch may be empty (N={document_count}, agents={agent_count}, cap={cap})",
            );
        }
    }

    #[test]
    fn chunk_evenly_with_cap_without_agents_splits_only_to_honor_the_cap() {
        let params = make_params(make_docs(600));

        let sub_batches = params.chunk_evenly_with_cap(0, chunk_cap(256));

        assert_eq!(sub_batch_sizes(&sub_batches), vec![200, 200, 200]);
    }

    #[test]
    fn chunk_evenly_with_cap_preserves_normalization_method() {
        let params = GenerateEmbeddingBatchParams {
            input_batch: make_docs(8),
            normalization_method: EmbeddingNormalizationMethod::L2,
        };

        let sub_batches = params.chunk_evenly_with_cap(4, chunk_cap(256));

        let is_l2 = |normalization_method: &EmbeddingNormalizationMethod| {
            matches!(normalization_method, EmbeddingNormalizationMethod::L2)
        };

        assert_eq!(sub_batches.len(), 4);
        assert!(sub_batches.iter().all(
            |GenerateEmbeddingBatchParams {
                 normalization_method,
                 ..
             }| is_l2(normalization_method)
        ));
        assert!(!is_l2(&EmbeddingNormalizationMethod::None));
    }
}
