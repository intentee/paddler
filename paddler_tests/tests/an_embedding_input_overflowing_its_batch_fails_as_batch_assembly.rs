#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU16;

use llama_cpp_bindings::batch_add_error::BatchAddError;
use llama_cpp_bindings::token::LlamaToken;
use llama_cpp_bindings_types::BatchCapacity;

use paddler_agent_embeddings::embedding_error::EmbeddingError;
use paddler_agent_embeddings::embedding_input_tokenized::EmbeddingInputTokenized;
use paddler_agent_embeddings::embedding_normalization::EmbeddingNormalization;
use paddler_inference_parameters::pooling_type::PoolingType;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_tests::active_embedding_request_fixture::ActiveEmbeddingRequestFixture;

use crate::nomic_embedding_batch_processing::NomicEmbeddingBatchProcessing;

const TOKENS_THE_BATCH_HOLDS: NonZeroU16 = NonZeroU16::MIN;

#[test]
fn an_embedding_input_overflowing_its_batch_fails_as_batch_assembly() {
    let nomic_embedding_batch_processing =
        NomicEmbeddingBatchProcessing::load(PoolingType::Mean, NonZeroU16::MIN);
    let ActiveEmbeddingRequestFixture {
        admitted_requests,
        generate_embedding_stop_tx: _generate_embedding_stop_tx,
        mut generated_embedding_rx,
    } = nomic_embedding_batch_processing.admit(vec![EmbeddingInputTokenized {
        id: "document".to_owned(),
        tokens: vec![LlamaToken::new(101), LlamaToken::new(102)],
    }]);

    let continuing_requests = nomic_embedding_batch_processing.embed_next_batch(
        BatchCapacity::from(TOKENS_THE_BATCH_HOLDS),
        EmbeddingNormalization,
        admitted_requests,
    );

    assert_eq!(continuing_requests.len(), 0);
    assert_eq!(
        generated_embedding_rx.try_recv(),
        Ok(EmbeddingResult::BatchAssemblyFailed(
            EmbeddingError::BatchAssemblyFailed(BatchAddError::InsufficientSpace(usize::from(
                TOKENS_THE_BATCH_HOLDS.get()
            )))
            .to_string()
        ))
    );
}
