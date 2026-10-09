use llama_cpp_bindings::model::AddBos;
use llama_cpp_bindings::model::ParseSpecialTokens;

use paddler_agent_runtime::agent_request::AgentRequest;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_agent_runtime::prepares_scheduler_command::PreparesSchedulerCommand;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::oversized_embedding_document_details::OversizedEmbeddingDocumentDetails;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;

use crate::embedding_error::EmbeddingError;
use crate::embedding_input_tokenized::EmbeddingInputTokenized;
use crate::prepared_embedding_batch_request::PreparedEmbeddingBatchRequest;

pub struct EmbeddingBatchPreparer {
    pub loaded_llama_model: LoadedLlamaModel,
    pub n_batch: usize,
}

impl EmbeddingBatchPreparer {
    fn prepare(
        &self,
        AgentRequest {
            params:
                GenerateEmbeddingBatchParams {
                    input_batch,
                    normalization_method,
                },
            response_tx: generated_embedding_tx,
            slot_guard,
            stop_rx: generate_embedding_stop_rx,
        }: AgentRequest<GenerateEmbeddingBatchParams, EmbeddingResult>,
    ) -> Result<PreparedEmbeddingBatchRequest, EmbeddingError> {
        let mut inputs = Vec::with_capacity(input_batch.len());
        let mut oversized_documents = Vec::new();

        for input in input_batch {
            let tokens = self
                .loaded_llama_model
                .model
                .str_to_token(&input.content, AddBos::Always, ParseSpecialTokens::Always)
                .map_err(|source| EmbeddingError::InputTokenizationFailed {
                    source_document_id: input.id.clone(),
                    source,
                })?;

            if tokens.len() > self.n_batch {
                oversized_documents.push(OversizedEmbeddingDocumentDetails {
                    document_tokens: tokens.len(),
                    n_batch: self.n_batch,
                    source_document_id: input.id,
                });
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
            oversized_documents,
            slot_guard,
        })
    }
}

impl PreparesSchedulerCommand for EmbeddingBatchPreparer {
    type Command = PreparedEmbeddingBatchRequest;
    type Request = AgentRequest<GenerateEmbeddingBatchParams, EmbeddingResult>;

    fn prepare_scheduler_command(
        &self,
        agent_name: Option<&str>,
        request: Self::Request,
    ) -> Option<Self::Command> {
        let generated_embedding_tx = request.response_tx.clone();

        match self.prepare(request) {
            Ok(prepared) => Some(prepared),
            Err(rejection) => {
                rejection.report(agent_name, &generated_embedding_tx);

                None
            }
        }
    }
}
