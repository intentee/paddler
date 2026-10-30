#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU16;

use llama_cpp_bindings::error::DecodeError;
use llama_cpp_bindings::token::LlamaToken;

use paddler_agent_embeddings::embedding_error::EmbeddingError;
use paddler_agent_embeddings::embedding_input_tokenized::EmbeddingInputTokenized;
use paddler_agent_embeddings::embedding_normalization::EmbeddingNormalization;
use paddler_inference_parameters::pooling_type::PoolingType;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_tests::active_embedding_request_fixture::ActiveEmbeddingRequestFixture;

use crate::nomic_embedding_batch_processing::NomicEmbeddingBatchProcessing;

#[test]
fn an_embedding_input_llama_cpp_rejects_fails_the_embedding_batch() {
    let nomic_embedding_batch_processing =
        NomicEmbeddingBatchProcessing::load(PoolingType::Mean, NonZeroU16::MIN);
    let ActiveEmbeddingRequestFixture {
        admitted_requests,
        generate_embedding_stop_tx: _generate_embedding_stop_tx,
        mut generated_embedding_rx,
    } = nomic_embedding_batch_processing.admit(vec![EmbeddingInputTokenized {
        id: "document".to_owned(),
        tokens: vec![LlamaToken::new(
            nomic_embedding_batch_processing
                .llama_runtime
                .loaded_llama_model
                .model
                .n_vocab(),
        )],
    }]);

    let continuing_requests = nomic_embedding_batch_processing.embed_next_batch(
        nomic_embedding_batch_processing.batch_capacity(),
        EmbeddingNormalization,
        admitted_requests,
    );

    assert_eq!(continuing_requests.len(), 0);
    assert_eq!(
        generated_embedding_rx.try_recv(),
        Ok(EmbeddingResult::DecodeFailed(
            EmbeddingError::DecodeFailed(DecodeError::BatchInvalid).to_string()
        ))
    );
}
