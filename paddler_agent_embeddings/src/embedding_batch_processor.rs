use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use log::warn;
use tokio::sync::mpsc;

use paddler_agent_runtime::receives_stop_request::ReceivesStopRequest as _;
use paddler_messaging::embedding::Embedding;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::embedding_result::EmbeddingResult;

use crate::embedding_error::EmbeddingError;
use crate::embedding_scheduler_context::EmbeddingSchedulerContext;
use crate::normalization::normalize_embedding::normalize_embedding;
use crate::plan_embedding_batches::plan_embedding_batches;
use crate::prepared_embedding_batch_request::PreparedEmbeddingBatchRequest;
use crate::sequenced_embedding_input::SequencedEmbeddingInput;

pub struct EmbeddingBatchProcessor<'scheduler, 'model> {
    pub batch: &'scheduler mut LlamaBatch<'static>,
    pub llama_context: &'scheduler mut LlamaContext<'model>,
    pub scheduler_context: &'scheduler EmbeddingSchedulerContext,
}

impl EmbeddingBatchProcessor<'_, '_> {
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
    ) -> Result<(), EmbeddingError> {
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
                .map_err(EmbeddingError::ClientDisconnected)?;
        }

        let n_batch = self.scheduler_context.n_batch;
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

            let batch_inputs: Vec<SequencedEmbeddingInput> = (0..max_sequences_per_batch)
                .zip(&inputs[planned_batch])
                .map(|(sequence_id, input)| SequencedEmbeddingInput {
                    input,
                    sequence_id: i32::from(sequence_id),
                })
                .collect();

            for SequencedEmbeddingInput { input, sequence_id } in &batch_inputs {
                self.batch
                    .add_sequence(&input.tokens, *sequence_id, true)
                    .map_err(EmbeddingError::BatchAssemblyFailed)?;
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
            .map_err(EmbeddingError::ClientDisconnected)
    }

    fn embedding_batch_decode(
        &mut self,
        current_batch_embeddings: &[SequencedEmbeddingInput],
        generated_embedding_tx: &mpsc::UnboundedSender<EmbeddingResult>,
        normalization_method: &EmbeddingNormalizationMethod,
    ) -> Result<(), EmbeddingError> {
        self.llama_context.clear_kv_cache();
        self.llama_context
            .decode(self.batch)
            .map_err(EmbeddingError::DecodeFailed)?;

        for SequencedEmbeddingInput { input, sequence_id } in current_batch_embeddings {
            let embedding = self
                .llama_context
                .embeddings_seq_ith(*sequence_id)
                .map_err(EmbeddingError::EmbeddingsUnavailable)?;

            generated_embedding_tx
                .send(EmbeddingResult::Embedding(Embedding {
                    embedding: normalize_embedding(embedding.to_vec(), normalization_method)?,
                    normalization_method: normalization_method.clone(),
                    pooling_type: self.scheduler_context.pooling_type.clone(),
                    source_document_id: input.id.clone(),
                }))
                .map_err(EmbeddingError::ClientDisconnected)?;
        }

        self.batch.clear();

        Ok(())
    }
}
