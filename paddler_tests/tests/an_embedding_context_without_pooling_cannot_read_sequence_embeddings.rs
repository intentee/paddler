#![cfg(feature = "tests_that_use_llms")]

use hf_hub::Cache;
use llama_cpp_bindings::error::EmbeddingsError;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::token::LlamaToken;
use tokio::sync::mpsc;

use paddler_agent_embeddings::embedding_batch_processor::EmbeddingBatchProcessor;
use paddler_agent_embeddings::embedding_error::EmbeddingError;
use paddler_agent_embeddings::embedding_input_tokenized::EmbeddingInputTokenized;
use paddler_agent_embeddings::embedding_normalization::EmbeddingNormalization;
use paddler_agent_embeddings::embedding_scheduler_context::EmbeddingSchedulerContext;
use paddler_agent_embeddings::prepared_embedding_batch_request::PreparedEmbeddingBatchRequest;
use paddler_agent_status::slot_guard::SlotGuard;
use paddler_inference_parameters::pooling_type::PoolingType;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_test_cluster_harness::model_card::nomic_embed_text_v1_5::nomic_embed_text_v1_5;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::llama_runtime::LlamaRuntime;

#[test]
fn an_embedding_context_without_pooling_cannot_read_sequence_embeddings() {
    let runtime = LlamaRuntime::load(
        &nomic_embed_text_v1_5()
            .reference
            .cached_path(&Cache::from_env())
            .expect("the nomic embedding model must be in the Hugging Face cache"),
        agent_inference_runtime_context(1),
    )
    .expect("the nomic embedding model must load");
    let mut llama_context = runtime
        .create_embedding_llama_context(1, PoolingType::None)
        .expect("the embedding context must be created");
    let mut batch = LlamaBatch::new(runtime.model_runtime_parameters.n_batch.capacity(), 1);
    let (generated_embedding_tx, _generated_embedding_rx) = mpsc::unbounded_channel();
    let (_generate_embedding_stop_tx, generate_embedding_stop_rx) = mpsc::unbounded_channel();

    assert_eq!(
        EmbeddingBatchProcessor {
            batch: &mut batch,
            llama_context: &mut llama_context,
            normalization: EmbeddingNormalization,
            scheduler_context: &EmbeddingSchedulerContext {
                agent_name: None,
                desired_slots_total: 1,
                n_batch: runtime.model_runtime_parameters.n_batch.tokens_usize(),
                pooling_type: PoolingType::None,
            },
        }
        .process_embedding_batch(PreparedEmbeddingBatchRequest {
            generate_embedding_stop_rx,
            generated_embedding_tx,
            inputs: vec![EmbeddingInputTokenized {
                id: "document".to_owned(),
                tokens: vec![LlamaToken::new(101), LlamaToken::new(102)],
            }],
            normalization_method: EmbeddingNormalizationMethod::None,
            oversized_documents: Vec::new(),
            slot_guard: SlotGuard::new(
                runtime
                    .inference_runtime_context
                    .slot_aggregated_status
                    .clone()
            ),
        })
        .err()
        .as_ref()
        .map(ToString::to_string),
        Some(EmbeddingError::EmbeddingsUnavailable(EmbeddingsError::NonePoolType).to_string())
    );
}
