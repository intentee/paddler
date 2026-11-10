use std::num::NonZeroU16;

use anyhow::Context as _;
use anyhow::Result;
use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::context::context_limits::ContextLimits;
use llama_cpp_bindings::llama_batch::LlamaBatch;

use paddler_agent_embeddings::active_embedding_request::ActiveEmbeddingRequest;
use paddler_agent_embeddings::embedding_batch_processor::EmbeddingBatchProcessor;
use paddler_agent_embeddings::embedding_input_tokenized::EmbeddingInputTokenized;
use paddler_agent_embeddings::embedding_scheduler_context::EmbeddingSchedulerContext;
use paddler_agent_embeddings::normalizes_embedding::NormalizesEmbedding;
use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_agent_runtime::resident_llama_context::ResidentLlamaContext;
use paddler_inference_parameters::pooling_type::PoolingType;
use paddler_test_cluster_harness::model_card::nomic_embed_text_v1_5::nomic_embed_text_v1_5;

use crate::active_embedding_request_fixture::ActiveEmbeddingRequestFixture;
use crate::agent_inference_runtime_context::agent_inference_runtime_context;
use crate::llama_runtime::LlamaRuntime;

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
    pub fn admit(&self, inputs: Vec<EmbeddingInputTokenized>) -> ActiveEmbeddingRequestFixture {
        ActiveEmbeddingRequestFixture::new(
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
    /// Returns the [`AgentRuntimeError`] of [`Self::embedding_llama_context`] when the embedding context cannot be created.
    pub fn embed_next_batch<TNormalization: NormalizesEmbedding>(
        &self,
        normalization: TNormalization,
        active_requests: Vec<ActiveEmbeddingRequest>,
    ) -> Result<Vec<ActiveEmbeddingRequest>, AgentRuntimeError> {
        self.embedding_llama_context().map(|mut llama_context| {
            self.embed_next_batch_in(&mut llama_context, normalization, active_requests)
        })
    }

    pub fn embed_next_batch_in<TNormalization: NormalizesEmbedding>(
        &self,
        llama_context: &mut LlamaContext<'_>,
        normalization: TNormalization,
        active_requests: Vec<ActiveEmbeddingRequest>,
    ) -> Vec<ActiveEmbeddingRequest> {
        let mut batch = LlamaBatch::new(ContextLimits::of(llama_context));

        EmbeddingBatchProcessor {
            batch: &mut batch,
            llama_context,
            normalization,
            scheduler_context: &EmbeddingSchedulerContext {
                agent_name: None,
                pooling_type: self.pooling_type.clone(),
            },
        }
        .embed_next_batch(active_requests)
    }

    /// # Errors
    ///
    /// Returns [`AgentRuntimeError::LlamaContextAlreadyResident`] when an embedding context already occupies the VRAM, and [`AgentRuntimeError::ContextCreationFailed`] when llama.cpp cannot create the context.
    pub fn embedding_llama_context(&self) -> Result<ResidentLlamaContext<'_>, AgentRuntimeError> {
        self.llama_runtime.create_embedding_llama_context(
            u32::from(self.sequences.get()),
            self.pooling_type.clone(),
        )
    }
}
