use crate::active_embedding_request::ActiveEmbeddingRequest;
use crate::batched_embedding_input::BatchedEmbeddingInput;

#[must_use]
pub fn next_embedding_batch(
    active_requests: &mut [ActiveEmbeddingRequest],
    n_batch: usize,
    sequences_in_batch: u16,
) -> Vec<BatchedEmbeddingInput> {
    let mut batched_inputs = Vec::new();
    let mut batched_tokens = 0;
    let mut next_sequence_id: u16 = 0;

    for (request_index, active_request) in active_requests.iter_mut().enumerate() {
        let mut request_sequences = 0;

        while request_sequences < active_request.sequences()
            && next_sequence_id < sequences_in_batch
        {
            let Some(input) = active_request.pending_inputs.pop_front() else {
                break;
            };

            if batched_tokens + input.tokens.len() > n_batch {
                active_request.pending_inputs.push_front(input);

                break;
            }

            batched_tokens += input.tokens.len();
            batched_inputs.push(BatchedEmbeddingInput {
                input,
                request_index,
                sequence_id: i32::from(next_sequence_id),
            });
            request_sequences += 1;
            next_sequence_id += 1;
        }
    }

    batched_inputs
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::num::NonZeroU16;
    use std::sync::Arc;

    use llama_cpp_bindings::token::LlamaToken;
    use tokio::sync::mpsc;

    use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
    use paddler_agent_status::slot_guard::SlotGuard;
    use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;

    use super::next_embedding_batch;
    use crate::active_embedding_request::ActiveEmbeddingRequest;
    use crate::batched_embedding_input::BatchedEmbeddingInput;
    use crate::embedding_input_tokenized::EmbeddingInputTokenized;
    use crate::prepared_embedding_batch_request::PreparedEmbeddingBatchRequest;

    fn document(id: &str, tokens: usize) -> EmbeddingInputTokenized {
        EmbeddingInputTokenized {
            id: id.to_owned(),
            tokens: vec![LlamaToken::new(0); tokens],
        }
    }

    fn active_request(
        sequences: u16,
        inputs: Vec<EmbeddingInputTokenized>,
    ) -> ActiveEmbeddingRequest {
        let (generated_embedding_tx, _generated_embedding_rx) = mpsc::unbounded_channel();
        let (_generate_embedding_stop_tx, generate_embedding_stop_rx) = mpsc::unbounded_channel();

        PreparedEmbeddingBatchRequest {
            generate_embedding_stop_rx,
            generated_embedding_tx,
            inputs,
            normalization_method: EmbeddingNormalizationMethod::None,
            unembeddable_documents: Vec::new(),
            slot_guard: SlotGuard::new(
                Arc::new(SlotAggregatedStatus::new(8)),
                NonZeroU16::new(sequences).unwrap(),
            ),
        }
        .admit(None)
        .into_iter()
        .next()
        .expect("a request with inputs to embed must stay active")
    }

    fn batched(
        request_index: usize,
        input: EmbeddingInputTokenized,
        sequence_id: i32,
    ) -> BatchedEmbeddingInput {
        BatchedEmbeddingInput {
            input,
            request_index,
            sequence_id,
        }
    }

    #[test]
    fn every_request_contributes_up_to_its_reserved_sequences() {
        let mut active_requests = [
            active_request(1, vec![document("a1", 1), document("a2", 1)]),
            active_request(
                2,
                vec![document("b1", 1), document("b2", 1), document("b3", 1)],
            ),
        ];

        assert_eq!(
            next_embedding_batch(&mut active_requests, 100, 8),
            [
                batched(0, document("a1", 1), 0),
                batched(1, document("b1", 1), 1),
                batched(1, document("b2", 1), 2),
            ]
        );
        assert_eq!(
            active_requests[1].pending_inputs,
            VecDeque::from([document("b3", 1)])
        );
    }

    #[test]
    fn a_request_without_enough_documents_leaves_its_reserved_sequences_unused() {
        let mut active_requests = [
            active_request(3, vec![document("a1", 1)]),
            active_request(1, vec![document("b1", 1)]),
        ];

        assert_eq!(
            next_embedding_batch(&mut active_requests, 100, 8),
            [
                batched(0, document("a1", 1), 0),
                batched(1, document("b1", 1), 1)
            ]
        );
    }

    #[test]
    fn a_document_that_does_not_fit_the_remaining_tokens_waits_for_the_next_batch() {
        let mut active_requests = [
            active_request(1, vec![document("a1", 6)]),
            active_request(2, vec![document("b1", 5), document("b2", 3)]),
        ];

        assert_eq!(
            next_embedding_batch(&mut active_requests, 10, 8),
            [batched(0, document("a1", 6), 0)]
        );
        assert_eq!(
            active_requests[1].pending_inputs,
            VecDeque::from([document("b1", 5), document("b2", 3)])
        );
    }

    #[test]
    fn a_batch_holds_no_more_sequences_than_the_context_serves() {
        let mut active_requests = [
            active_request(2, vec![document("a1", 1), document("a2", 1)]),
            active_request(2, vec![document("b1", 1), document("b2", 1)]),
        ];

        assert_eq!(
            next_embedding_batch(&mut active_requests, 100, 3),
            [
                batched(0, document("a1", 1), 0),
                batched(0, document("a2", 1), 1),
                batched(1, document("b1", 1), 2),
            ]
        );
    }
}
