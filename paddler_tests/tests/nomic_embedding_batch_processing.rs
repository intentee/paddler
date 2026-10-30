#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU16;

use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings_types::BatchCapacity;

use paddler_agent_embeddings::active_embedding_request::ActiveEmbeddingRequest;
use paddler_agent_embeddings::embedding_batch_processor::EmbeddingBatchProcessor;
use paddler_agent_embeddings::embedding_input_tokenized::EmbeddingInputTokenized;
use paddler_agent_embeddings::embedding_scheduler_context::EmbeddingSchedulerContext;
use paddler_agent_embeddings::normalizes_embedding::NormalizesEmbedding;
use paddler_agent_runtime::resident_llama_context::ResidentLlamaContext;
use paddler_inference_parameters::pooling_type::PoolingType;
use paddler_test_cluster_harness::model_card::nomic_embed_text_v1_5::nomic_embed_text_v1_5;
use paddler_tests::active_embedding_request_fixture::ActiveEmbeddingRequestFixture;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::llama_runtime::LlamaRuntime;

pub struct NomicEmbeddingBatchProcessing {
    pub llama_runtime: LlamaRuntime,
    pub pooling_type: PoolingType,
    pub sequences: NonZeroU16,
}

impl NomicEmbeddingBatchProcessing {
    pub fn load(pooling_type: PoolingType, sequences: NonZeroU16) -> Self {
        Self {
            llama_runtime: LlamaRuntime::load(
                &nomic_embed_text_v1_5()
                    .cached_path()
                    .expect("the nomic embedding model must be in the Hugging Face cache"),
                agent_inference_runtime_context(sequences.get()),
            )
            .expect("the nomic embedding model must load"),
            pooling_type,
            sequences,
        }
    }

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

    pub fn batch_capacity(&self) -> BatchCapacity {
        self.llama_runtime
            .model_runtime_parameters
            .n_batch
            .capacity()
    }

    pub fn embedding_llama_context(&self) -> ResidentLlamaContext<'_> {
        self.llama_runtime
            .create_embedding_llama_context(
                u32::from(self.sequences.get()),
                self.pooling_type.clone(),
            )
            .expect("the embedding context must be created")
    }

    pub fn embed_next_batch<TNormalization: NormalizesEmbedding>(
        &self,
        batch_capacity: BatchCapacity,
        normalization: TNormalization,
        active_requests: Vec<ActiveEmbeddingRequest>,
    ) -> Vec<ActiveEmbeddingRequest> {
        self.embed_next_batch_in(
            &mut self.embedding_llama_context(),
            batch_capacity,
            normalization,
            active_requests,
        )
    }

    pub fn embed_next_batch_in<TNormalization: NormalizesEmbedding>(
        &self,
        llama_context: &mut LlamaContext<'_>,
        batch_capacity: BatchCapacity,
        normalization: TNormalization,
        active_requests: Vec<ActiveEmbeddingRequest>,
    ) -> Vec<ActiveEmbeddingRequest> {
        let mut batch = LlamaBatch::new(batch_capacity, i32::from(self.sequences.get()));

        EmbeddingBatchProcessor {
            batch: &mut batch,
            llama_context,
            normalization,
            scheduler_context: &EmbeddingSchedulerContext {
                agent_name: None,
                desired_slots_total: self.sequences.get(),
                n_batch: self
                    .llama_runtime
                    .model_runtime_parameters
                    .n_batch
                    .tokens_usize(),
                pooling_type: self.pooling_type.clone(),
            },
        }
        .embed_next_batch(active_requests)
    }
}
