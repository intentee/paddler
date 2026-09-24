use std::sync::Arc;

use anyhow::Context as _;
use anyhow::Result;
use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use paddler_messaging::embedding::Embedding;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::embedding_result::EmbeddingResult;
use tokio::sync::mpsc;

use crate::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use crate::embedding_input_tokenized::EmbeddingInputTokenized;
use crate::normalization::normalize_embedding::normalize_embedding;
use crate::plan_embedding_batches::plan_embedding_batches;
use crate::prepared_embedding_batch_request::PreparedEmbeddingBatchRequest;

pub struct ContinuousBatchEmbeddingProcessor<'context> {
    batch: &'context mut LlamaBatch<'static>,
    llama_context: &'context mut LlamaContext<'static>,
    scheduler_context: &'context Arc<ContinuousBatchSchedulerContext>,
}

impl<'context> ContinuousBatchEmbeddingProcessor<'context> {
    pub const fn new(
        batch: &'context mut LlamaBatch<'static>,
        llama_context: &'context mut LlamaContext<'static>,
        scheduler_context: &'context Arc<ContinuousBatchSchedulerContext>,
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
            slot_guard,
        }: PreparedEmbeddingBatchRequest,
    ) -> Result<()> {
        let _slot_guard = slot_guard;

        let n_batch = self.scheduler_context.inference_parameters.n_batch;
        let max_sequences_per_batch = self.scheduler_context.desired_slots_total;

        let token_counts: Vec<usize> = inputs.iter().map(|input| input.tokens.len()).collect();
        let planned_batches =
            plan_embedding_batches(&token_counts, n_batch, max_sequences_per_batch);
        self.batch.clear();

        let mut embeddings_emitted: usize = 0;

        for planned_batch in planned_batches {
            if generate_embedding_stop_rx.try_recv().is_ok() {
                break;
            }

            let batch_inputs: Vec<&EmbeddingInputTokenized> =
                inputs[planned_batch].iter().collect();

            for (sequence_index, input) in batch_inputs.iter().enumerate() {
                self.batch.add_sequence(
                    &input.tokens,
                    i32::try_from(sequence_index).context("sequence index does not fit in i32")?,
                    true,
                )?;
            }

            self.embedding_batch_decode(
                &batch_inputs,
                &generated_embedding_tx,
                &normalization_method,
            )?;

            embeddings_emitted += batch_inputs.len();
        }

        if embeddings_emitted == 0 {
            generated_embedding_tx.send(EmbeddingResult::NoEmbeddingsProduced)?;
        } else {
            generated_embedding_tx.send(EmbeddingResult::Done)?;
        }

        Ok(())
    }

    fn embedding_batch_decode(
        &mut self,
        current_batch_embeddings: &[&EmbeddingInputTokenized],
        generated_embedding_tx: &mpsc::UnboundedSender<EmbeddingResult>,
        normalization_method: &EmbeddingNormalizationMethod,
    ) -> Result<()> {
        self.llama_context.clear_kv_cache();
        self.llama_context.decode(self.batch)?;

        for (index, embedding_input_tokenized) in current_batch_embeddings.iter().enumerate() {
            let embedding = self
                .llama_context
                .embeddings_seq_ith(
                    i32::try_from(index).context("embedding sequence index does not fit in i32")?,
                )
                .context("Failed to get embeddings")?;

            generated_embedding_tx.send(EmbeddingResult::Embedding(normalize_embedding(
                Embedding {
                    embedding: embedding.to_vec(),
                    normalization_method: EmbeddingNormalizationMethod::None,
                    pooling_type: self
                        .scheduler_context
                        .inference_parameters
                        .pooling_type
                        .clone(),
                    source_document_id: embedding_input_tokenized.id.clone(),
                },
                normalization_method,
            )?))?;
        }

        self.batch.clear();

        Ok(())
    }
}
