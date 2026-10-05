use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use log::warn;
use tokio::sync::mpsc;

use paddler_messaging::embedding::Embedding;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::embedding_result::EmbeddingResult;

use crate::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use crate::embedding_batch_rejection::EmbeddingBatchRejection;
use crate::embedding_input_tokenized::EmbeddingInputTokenized;
use crate::normalization::normalize_embedding::normalize_embedding;
use crate::plan_embedding_batches::plan_embedding_batches;
use crate::prepared_embedding_batch_request::PreparedEmbeddingBatchRequest;
use crate::receives_stop_request::ReceivesStopRequest as _;

pub struct ContinuousBatchEmbeddingProcessor<'context> {
    batch: &'context mut LlamaBatch<'static>,
    llama_context: &'context mut LlamaContext<'static>,
    scheduler_context: &'context ContinuousBatchSchedulerContext,
}

impl<'context> ContinuousBatchEmbeddingProcessor<'context> {
    pub const fn new(
        batch: &'context mut LlamaBatch<'static>,
        llama_context: &'context mut LlamaContext<'static>,
        scheduler_context: &'context ContinuousBatchSchedulerContext,
    ) -> Self {
        Self {
            batch,
            llama_context,
            scheduler_context,
        }
    }

    pub fn process_embedding_batch(
        &mut self,
        PreparedEmbeddingBatchRequest {
            mut generate_embedding_stop_rx,
            generated_embedding_tx,
            inputs,
            normalization_method,
            oversized_documents,
            slot_guard,
        }: PreparedEmbeddingBatchRequest,
    ) -> Result<(), EmbeddingBatchRejection> {
        let _slot_guard = slot_guard;

        for oversized_document in oversized_documents {
            warn!(
                "{:?}: skipped embedding document {:?}: {} tokens exceeds n_batch {}",
                self.scheduler_context.agent_name,
                oversized_document.source_document_id,
                oversized_document.document_tokens,
                oversized_document.n_batch,
            );

            generated_embedding_tx
                .send(EmbeddingResult::DocumentExceedsBatchSize(
                    oversized_document,
                ))
                .map_err(EmbeddingBatchRejection::ClientDisconnected)?;
        }

        let n_batch = self
            .scheduler_context
            .inference_parameters
            .n_batch
            .tokens_usize();
        let max_sequences_per_batch = self.scheduler_context.desired_slots_total;

        let token_counts: Vec<usize> = inputs.iter().map(|input| input.tokens.len()).collect();
        let planned_batches =
            plan_embedding_batches(&token_counts, n_batch, max_sequences_per_batch);
        self.batch.clear();

        let mut embeddings_emitted: usize = 0;

        for planned_batch in planned_batches {
            if generate_embedding_stop_rx.is_stop_requested() {
                break;
            }

            let batch_inputs: Vec<&EmbeddingInputTokenized> =
                inputs[planned_batch].iter().collect();

            for (sequence_index, input) in batch_inputs.iter().enumerate() {
                self.batch
                    .add_sequence(
                        &input.tokens,
                        i32::try_from(sequence_index)
                            .map_err(EmbeddingBatchRejection::SequenceIndexOutOfRange)?,
                        true,
                    )
                    .map_err(EmbeddingBatchRejection::BatchAssemblyFailed)?;
            }

            self.embedding_batch_decode(
                &batch_inputs,
                &generated_embedding_tx,
                &normalization_method,
            )?;

            embeddings_emitted += batch_inputs.len();
        }

        generated_embedding_tx
            .send(if embeddings_emitted == 0 {
                EmbeddingResult::NoEmbeddingsProduced
            } else {
                EmbeddingResult::Done
            })
            .map_err(EmbeddingBatchRejection::ClientDisconnected)
    }

    fn embedding_batch_decode(
        &mut self,
        current_batch_embeddings: &[&EmbeddingInputTokenized],
        generated_embedding_tx: &mpsc::UnboundedSender<EmbeddingResult>,
        normalization_method: &EmbeddingNormalizationMethod,
    ) -> Result<(), EmbeddingBatchRejection> {
        self.llama_context.clear_kv_cache();
        self.llama_context
            .decode(self.batch)
            .map_err(EmbeddingBatchRejection::DecodeFailed)?;

        for (index, embedding_input_tokenized) in current_batch_embeddings.iter().enumerate() {
            let embedding = self
                .llama_context
                .embeddings_seq_ith(
                    i32::try_from(index)
                        .map_err(EmbeddingBatchRejection::SequenceIndexOutOfRange)?,
                )
                .map_err(EmbeddingBatchRejection::EmbeddingsUnavailable)?;

            generated_embedding_tx
                .send(EmbeddingResult::Embedding(Embedding {
                    embedding: normalize_embedding(embedding.to_vec(), normalization_method)?,
                    normalization_method: normalization_method.clone(),
                    pooling_type: self
                        .scheduler_context
                        .inference_parameters
                        .pooling_type
                        .clone(),
                    source_document_id: embedding_input_tokenized.id.clone(),
                }))
                .map_err(EmbeddingBatchRejection::ClientDisconnected)?;
        }

        self.batch.clear();

        Ok(())
    }
}
