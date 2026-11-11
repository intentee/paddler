use std::collections::BTreeSet;

use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::llama_batch::LlamaBatch;

use paddler_messaging::embedding::Embedding;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::embedding_vector::EmbeddingVector;

use crate::active_embedding_request::ActiveEmbeddingRequest;
use crate::batched_embedding_input::BatchedEmbeddingInput;
use crate::embedding_error::EmbeddingError;
use crate::embedding_input_tokenized::EmbeddingInputTokenized;
use crate::embedding_normalization::EmbeddingNormalization;
use crate::embedding_request_continuation::EmbeddingRequestContinuation;
use crate::embedding_request_step::EmbeddingRequestStep;
use crate::embedding_scheduler_context::EmbeddingSchedulerContext;
use crate::next_embedding_batch::next_embedding_batch;
use crate::normalizes_embedding::NormalizesEmbedding;

pub struct EmbeddingBatchProcessor<
    'scheduler,
    'model,
    TNormalization: NormalizesEmbedding = EmbeddingNormalization,
> {
    pub batch: &'scheduler mut LlamaBatch,
    pub llama_context: &'scheduler mut LlamaContext<'model>,
    pub normalization: TNormalization,
    pub scheduler_context: &'scheduler EmbeddingSchedulerContext,
}

impl<TNormalization: NormalizesEmbedding> EmbeddingBatchProcessor<'_, '_, TNormalization> {
    #[must_use]
    pub fn embed_next_batch(
        &mut self,
        active_requests: Vec<ActiveEmbeddingRequest>,
    ) -> Vec<ActiveEmbeddingRequest> {
        let agent_name = self.scheduler_context.agent_name.as_deref();
        let mut active_requests: Vec<ActiveEmbeddingRequest> = active_requests
            .into_iter()
            .flat_map(|active_request| {
                EmbeddingRequestContinuation::unless_stop_requested(active_request, agent_name)
            })
            .collect();
        let limits = self.batch.limits();
        let batched_inputs = next_embedding_batch(
            &mut active_requests,
            limits.n_batch as usize,
            limits.n_seq_max,
        );

        if batched_inputs.is_empty() {
            return active_requests;
        }

        let mut request_steps = vec![EmbeddingRequestStep::Continuing; active_requests.len()];

        match self.decode(&batched_inputs) {
            Ok(()) => {
                self.deliver_embeddings(batched_inputs, &mut active_requests, &mut request_steps);
            }
            Err(batch_error) => {
                let failed_request_indexes: BTreeSet<usize> = batched_inputs
                    .iter()
                    .map(|&BatchedEmbeddingInput { request_index, .. }| request_index)
                    .collect();

                batch_error.report_to_each(
                    agent_name,
                    active_requests
                        .iter()
                        .enumerate()
                        .filter(|(request_index, _active_request)| {
                            failed_request_indexes.contains(request_index)
                        })
                        .map(|(_request_index, active_request)| {
                            &active_request.generated_embedding_tx
                        }),
                );

                request_steps
                    .iter_mut()
                    .enumerate()
                    .filter(|(request_index, _request_step)| {
                        failed_request_indexes.contains(request_index)
                    })
                    .for_each(|(_request_index, request_step)| {
                        *request_step = EmbeddingRequestStep::Ended;
                    });
            }
        }

        active_requests
            .into_iter()
            .zip(request_steps)
            .filter(|(_active_request, request_step)| {
                *request_step == EmbeddingRequestStep::Continuing
            })
            .flat_map(|(active_request, _request_step)| {
                EmbeddingRequestContinuation::unless_every_input_is_embedded(
                    active_request,
                    agent_name,
                )
            })
            .collect()
    }

    fn decode(&mut self, batched_inputs: &[BatchedEmbeddingInput]) -> Result<(), EmbeddingError> {
        self.batch.clear();

        for BatchedEmbeddingInput {
            input, sequence_id, ..
        } in batched_inputs
        {
            self.batch
                .add_sequence(&input.tokens, *sequence_id, true)
                .map_err(EmbeddingError::BatchAssemblyFailed)?;
        }

        self.llama_context.memory_clear();
        self.llama_context
            .decode(self.batch)
            .map_err(EmbeddingError::DecodeFailed)
    }

    fn deliver_embeddings(
        &self,
        batched_inputs: Vec<BatchedEmbeddingInput>,
        active_requests: &mut [ActiveEmbeddingRequest],
        request_steps: &mut [EmbeddingRequestStep],
    ) {
        let agent_name = self.scheduler_context.agent_name.as_deref();
        let mut batched_inputs = batched_inputs.into_iter().peekable();

        for (request_index, (active_request, request_step)) in active_requests
            .iter_mut()
            .zip(request_steps.iter_mut())
            .enumerate()
        {
            while let Some(BatchedEmbeddingInput {
                input, sequence_id, ..
            }) = batched_inputs.next_if(
                |&BatchedEmbeddingInput {
                     request_index: batched_request_index,
                     ..
                 }| batched_request_index == request_index,
            ) {
                if *request_step == EmbeddingRequestStep::Ended {
                    continue;
                }

                let delivery = self
                    .embedding_of(&input, sequence_id, &active_request.normalization_method)
                    .and_then(|embedding| {
                        active_request
                            .generated_embedding_tx
                            .send(EmbeddingResult::Embedding(embedding))
                            .map_err(EmbeddingError::ClientDisconnected)
                    });

                match delivery {
                    Ok(()) => active_request.emitted_an_embedding = true,
                    Err(delivery_error) => {
                        delivery_error.report(agent_name, &active_request.generated_embedding_tx);
                        *request_step = EmbeddingRequestStep::Ended;
                    }
                }
            }
        }
    }

    fn embedding_of(
        &self,
        EmbeddingInputTokenized { id, .. }: &EmbeddingInputTokenized,
        sequence_id: i32,
        normalization_method: &EmbeddingNormalizationMethod,
    ) -> Result<Embedding, EmbeddingError> {
        let embedding = self
            .llama_context
            .embeddings_seq_ith(sequence_id)
            .map_err(EmbeddingError::EmbeddingsUnavailable)?;

        Ok(Embedding {
            vector: EmbeddingVector::from(
                self.normalization
                    .normalize(embedding.to_vec(), normalization_method)?,
            ),
            normalization_method: normalization_method.clone(),
            pooling_type: self.scheduler_context.pooling_type.clone(),
            source_document_id: id.clone(),
        })
    }
}
