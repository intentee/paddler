use std::sync::Arc;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::model::AddBos;
use paddler_types::embedding::Embedding;
use paddler_types::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_types::embedding_result::EmbeddingResult;
use paddler_types::request_params::GenerateEmbeddingBatchParams;
use tokio::sync::mpsc;

use crate::agent::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use crate::agent::generate_embedding_batch_request::GenerateEmbeddingBatchRequest;
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
        }: GenerateEmbeddingBatchRequest,
    ) -> Result<()> {
        if !self
            .scheduler_context
            .inference_parameters
            .enable_embeddings
        {
            generated_embedding_tx.send(EmbeddingResult::Error(
                "Embeddings are not enabled for this agent".to_owned(),
            ))?;

            return Err(anyhow!("Embeddings are not enabled"));
        }

        self.llama_context.clear_kv_cache();

        let tokens_lines_list = input_batch
            .into_iter()
            .map(|input| {
                match self
                    .scheduler_context
                    .model
                    .str_to_token(&input.content, AddBos::Always)
                {
                    Ok(llama_tokens) => Ok(EmbeddingInputTokenized {
                        id: input.id,
                        llama_tokens,
                    }),
                    Err(err) => Err(anyhow!("Failed to tokenize input: {err:?}")),
                }
            })
            .collect::<Result<Vec<EmbeddingInputTokenized>, _>>()
            .context("failed to tokenize embedding input batch")?;

        let batch_n_tokens = self.scheduler_context.inference_parameters.batch_n_tokens;

        #[expect(
            clippy::cast_possible_wrap,
            reason = "embedding_n_seq_max fits in i32 for llama.cpp FFI"
        )]
        let embedding_n_seq_max = self
            .scheduler_context
            .inference_parameters
            .embedding_n_seq_max as i32;
        let mut batch = LlamaBatch::new(batch_n_tokens, embedding_n_seq_max)?;
        let mut current_batch_inputs: Vec<&EmbeddingInputTokenized> = Vec::new();
        let mut current_batch_token_count: usize = 0;
        let mut next_seq_id: i32 = 0;

        for embedding_input_tokenized in &tokens_lines_list {
            if generate_embedding_stop_rx.try_recv().is_ok() {
                break;
            }

            let input_token_count = embedding_input_tokenized.llama_tokens.len();

            if (current_batch_token_count + input_token_count > batch_n_tokens
                || next_seq_id >= embedding_n_seq_max)
                && !current_batch_inputs.is_empty()
            {
                self.embedding_batch_decode(
                    &mut batch,
                    &current_batch_inputs,
                    &generated_embedding_tx,
                    &normalization_method,
                )?;

                current_batch_inputs.clear();
                current_batch_token_count = 0;
                next_seq_id = 0;
            }

            batch.add_sequence(&embedding_input_tokenized.llama_tokens, next_seq_id, true)?;

            current_batch_inputs.push(embedding_input_tokenized);
            current_batch_token_count += input_token_count;
            next_seq_id += 1;
        }

        if !current_batch_inputs.is_empty() {
            self.embedding_batch_decode(
                &mut batch,
                &current_batch_inputs,
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
