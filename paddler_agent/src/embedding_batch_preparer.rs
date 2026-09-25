use std::sync::Arc;

use llama_cpp_bindings::model::AddBos;
use llama_cpp_bindings::model::LlamaModel;
use log::warn;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::oversized_embedding_document_details::OversizedEmbeddingDocumentDetails;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;

use crate::embedding_batch_rejection::EmbeddingBatchRejection;
use crate::embedding_input_tokenized::EmbeddingInputTokenized;
use crate::generate_embedding_batch_request::GenerateEmbeddingBatchRequest;
use crate::prepared_embedding_batch_request::PreparedEmbeddingBatchRequest;
use crate::require_embeddings_enabled::require_embeddings_enabled;

pub struct EmbeddingBatchPreparer {
    pub enable_embeddings: bool,
    pub model: Arc<LlamaModel>,
    pub n_batch: usize,
}

impl EmbeddingBatchPreparer {
    pub fn prepare(
        &self,
        agent_name: Option<&str>,
        GenerateEmbeddingBatchRequest {
            generate_embedding_stop_rx,
            generated_embedding_tx,
            params:
                GenerateEmbeddingBatchParams {
                    input_batch,
                    normalization_method,
                },
            slot_guard,
        }: GenerateEmbeddingBatchRequest,
    ) -> Result<PreparedEmbeddingBatchRequest, EmbeddingBatchRejection> {
        require_embeddings_enabled(self.enable_embeddings)?;

        let mut inputs = Vec::with_capacity(input_batch.len());

        for input in input_batch {
            let tokens = self
                .model
                .str_to_token(&input.content, AddBos::Always)
                .map_err(|source| EmbeddingBatchRejection::InputTokenizationFailed {
                    source_document_id: input.id.clone(),
                    source,
                })?;

            if tokens.len() > self.n_batch {
                let details = OversizedEmbeddingDocumentDetails {
                    document_tokens: tokens.len(),
                    n_batch: self.n_batch,
                    source_document_id: input.id,
                };

                warn!(
                    "{agent_name:?}: skipped embedding document {:?}: {} tokens exceeds n_batch {}",
                    details.source_document_id, details.document_tokens, details.n_batch,
                );

                generated_embedding_tx
                    .send(EmbeddingResult::DocumentExceedsBatchSize(details))
                    .map_err(EmbeddingBatchRejection::ClientDisconnected)?;
            } else {
                inputs.push(EmbeddingInputTokenized {
                    id: input.id,
                    tokens,
                });
            }
        }

        Ok(PreparedEmbeddingBatchRequest {
            generate_embedding_stop_rx,
            generated_embedding_tx,
            inputs,
            normalization_method,
            slot_guard,
        })
    }
}
