use std::sync::Arc;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::model::AddBos;
use log::warn;
use paddler_types::embedding::Embedding;
use paddler_types::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_types::embedding_result::EmbeddingResult;
use paddler_types::oversized_embedding_document_details::OversizedEmbeddingDocumentDetails;
use paddler_types::request_params::GenerateEmbeddingBatchParams;
use tokio::sync::mpsc;

use crate::agent::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use crate::agent::generate_embedding_batch_request::GenerateEmbeddingBatchRequest;
use crate::agent::plan_embedding_batches::plan_embedding_batches;
use crate::embedding_input_tokenized::EmbeddingInputTokenized;

pub struct ContinuousBatchEmbeddingProcessor<'context> {
    llama_context: &'context mut LlamaContext<'static>,
    scheduler_context: &'context Arc<ContinuousBatchSchedulerContext>,
}

impl<'context> ContinuousBatchEmbeddingProcessor<'context> {
    pub const fn new(
        llama_context: &'context mut LlamaContext<'static>,
        scheduler_context: &'context Arc<ContinuousBatchSchedulerContext>,
    ) -> Self {
        Self {
            llama_context,
            scheduler_context,
        }
    }

    pub fn process_embedding_batch(
        &mut self,
        GenerateEmbeddingBatchRequest {
            mut generate_embedding_stop_rx,
            generated_embedding_tx,
            params:
                GenerateEmbeddingBatchParams {
                    input_batch,
                    normalization_method,
                },
            slot_guard,
        }: GenerateEmbeddingBatchRequest,
    ) -> Result<()> {
        #[expect(
            unused_variables,
            reason = "slot_guard is held until function returns to release the slot via Drop"
        )]
        let slot_guard = slot_guard;

        if !self
            .scheduler_context
            .inference_parameters
            .enable_embeddings
        {
            generated_embedding_tx.send(EmbeddingResult::EmbeddingsDisabled)?;

            return Err(anyhow!("Embeddings are not enabled"));
        }

        let tokens_lines_list = input_batch
            .into_iter()
            .map(|input| {
                match self
                    .scheduler_context
                    .model
                    .str_to_token(&input.content, AddBos::Always)
                {
                    Ok(tokens) => Ok(EmbeddingInputTokenized {
                        id: input.id,
                        tokens,
                    }),
                    Err(err) => Err(anyhow!("Failed to tokenize input: {err:?}")),
                }
            })
            .collect::<Result<Vec<EmbeddingInputTokenized>, _>>()
            .context("failed to tokenize embedding input batch")?;

        let n_batch = self.scheduler_context.inference_parameters.n_batch;
        let max_sequences_per_batch = self.scheduler_context.desired_slots_total;

        let mut tokens_lines_list_within_batch: Vec<EmbeddingInputTokenized> = Vec::new();
        for input in tokens_lines_list {
            if input.tokens.len() > n_batch {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "document token counts and n_batch are model-bounded and fit in u32"
                )]
                let details = OversizedEmbeddingDocumentDetails {
                    document_tokens: input.tokens.len() as u32,
                    n_batch: n_batch as u32,
                    source_document_id: input.id.clone(),
                };

                warn!(
                    "{:?}: skipped embedding document {:?}: {} tokens exceeds n_batch {}",
                    self.scheduler_context.agent_name,
                    input.id,
                    details.document_tokens,
                    details.n_batch,
                );

                generated_embedding_tx.send(EmbeddingResult::DocumentExceedsBatchSize(details))?;
            } else {
                tokens_lines_list_within_batch.push(input);
            }
        }

        let token_counts: Vec<usize> = tokens_lines_list_within_batch
            .iter()
            .map(|input| input.tokens.len())
            .collect();
        let planned_batches =
            plan_embedding_batches(&token_counts, n_batch, max_sequences_per_batch);
        let mut batch = LlamaBatch::new(n_batch, max_sequences_per_batch)?;

        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_possible_wrap,
            reason = "sequence index within a planned batch is bounded by max_sequences_per_batch which fits in i32"
        )]
        for planned_batch in planned_batches {
            if generate_embedding_stop_rx.try_recv().is_ok() {
                break;
            }

            let batch_inputs: Vec<&EmbeddingInputTokenized> = tokens_lines_list_within_batch
                [planned_batch]
                .iter()
                .collect();

            for (sequence_index, input) in batch_inputs.iter().enumerate() {
                batch.add_sequence(&input.tokens, sequence_index as i32, true)?;
            }

            self.embedding_batch_decode(
                &mut batch,
                &batch_inputs,
                &generated_embedding_tx,
                &normalization_method,
            )?;
        }

        generated_embedding_tx.send(EmbeddingResult::Done)?;

        Ok(())
    }

    fn embedding_batch_decode(
        &mut self,
        batch: &mut LlamaBatch,
        current_batch_embeddings: &[&EmbeddingInputTokenized],
        generated_embedding_tx: &mpsc::UnboundedSender<EmbeddingResult>,
        normalization_method: &EmbeddingNormalizationMethod,
    ) -> Result<()> {
        self.llama_context.clear_kv_cache();
        self.llama_context.decode(batch)?;

        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_possible_wrap,
            reason = "embedding sequence index fits in i32 for llama.cpp FFI"
        )]
        for (index, embedding_input_tokenized) in current_batch_embeddings.iter().enumerate() {
            let embedding = self
                .llama_context
                .embeddings_seq_ith(index as i32)
                .context("Failed to get embeddings")?;

            generated_embedding_tx.send(EmbeddingResult::Embedding(
                Embedding {
                    embedding: embedding.to_vec(),
                    normalization_method: EmbeddingNormalizationMethod::None,
                    pooling_type: self
                        .scheduler_context
                        .inference_parameters
                        .pooling_type
                        .clone(),
                    source_document_id: embedding_input_tokenized.id.clone(),
                }
                .normalize(normalization_method)?,
            ))?;
        }

        batch.clear();

        Ok(())
    }
}
