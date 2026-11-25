use std::num::NonZeroU16;

use anyhow::Context as _;
use anyhow::Result;
use llama_cpp_context::llama_batch::LlamaBatch;
use llama_cpp_context::llama_context::LlamaContext;

use paddler_agent_embeddings::active_embedding_request::ActiveEmbeddingRequest;
use paddler_agent_embeddings::embedding_batch_processor::EmbeddingBatchProcessor;
use paddler_agent_embeddings::embedding_error::EmbeddingError;
use paddler_agent_embeddings::embedding_input_tokenized::EmbeddingInputTokenized;
use paddler_agent_embeddings::embedding_scheduler_context::EmbeddingSchedulerContext;
use paddler_agent_pipeline_harness::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_agent_pipeline_harness::llama_runtime::LlamaRuntime;
use paddler_agent_runtime::resident_llama_context::ResidentLlamaContext;
use paddler_inference_parameters::pooling_type::PoolingType;
use paddler_model_card_fixture::nomic_embed_text_v1_5::nomic_embed_text_v1_5;

use crate::admitted_embedding_requests::AdmittedEmbeddingRequests;

pub struct NomicEmbeddingBatchProcessing {
    pub llama_runtime: LlamaRuntime,
    pub pooling_type: PoolingType,
    pub sequences: NonZeroU16,
}

impl NomicEmbeddingBatchProcessing {
    /// # Errors
    ///
    /// Returns an error when the nomic embedding model is not in the Hugging Face cache, and when it cannot load.
    pub fn load(pooling_type: PoolingType, sequences: NonZeroU16) -> Result<Self> {
        nomic_embed_text_v1_5()
            .cached_path()
            .context("the nomic embedding model must be in the Hugging Face cache")
            .and_then(|model_path| {
                LlamaRuntime::load(
                    &model_path,
                    agent_inference_runtime_context(sequences.get()),
                )
                .context("the nomic embedding model must load")
            })
            .map(|llama_runtime| Self {
                llama_runtime,
                pooling_type,
                sequences,
            })
    }

    #[must_use]
    pub fn admit(&self, inputs: Vec<EmbeddingInputTokenized>) -> AdmittedEmbeddingRequests {
        AdmittedEmbeddingRequests::new(
            inputs,
            self.llama_runtime
                .inference_runtime_context
                .slot_aggregated_status
                .clone(),
            self.sequences,
        )
    }

    /// # Errors
    ///
    /// Returns the [`EmbeddingError`] of [`Self::embedding_llama_context`] when the embedding context cannot be created.
    pub fn embed_next_batch(
        &self,
        active_requests: Vec<ActiveEmbeddingRequest>,
    ) -> Result<Vec<ActiveEmbeddingRequest>, EmbeddingError> {
        self.embedding_llama_context()
            .map(|mut llama_context| self.embed_next_batch_in(&mut llama_context, active_requests))
    }

    pub fn embed_next_batch_in(
        &self,
        llama_context: &mut LlamaContext<'_>,
        active_requests: Vec<ActiveEmbeddingRequest>,
    ) -> Vec<ActiveEmbeddingRequest> {
        let mut batch = LlamaBatch::new(llama_context.limits());

        EmbeddingBatchProcessor {
            batch: &mut batch,
            llama_context,
            scheduler_context: &EmbeddingSchedulerContext {
                agent_name: None,
                pooling_type: self.pooling_type.clone(),
            },
        }
        .embed_next_batch(active_requests)
    }

    /// # Errors
    ///
    /// Returns [`EmbeddingError::AgentRuntime`] when llama.cpp cannot create the embedding context or an embedding context already occupies the VRAM.
    pub fn embedding_llama_context(&self) -> Result<ResidentLlamaContext<'_>, EmbeddingError> {
        self.llama_runtime.create_embedding_llama_context(
            u32::from(self.sequences.get()),
            self.pooling_type.clone(),
        )
    }
}
