#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU16;

use llama_cpp_bindings::batch_add_error::BatchAddError;
use llama_cpp_bindings::error::token_outside_vocabulary::TokenOutsideVocabulary;
use llama_cpp_bindings::token::LlamaToken;

use paddler_agent_embeddings::embedding_error::EmbeddingError;
use paddler_agent_embeddings::embedding_input_tokenized::EmbeddingInputTokenized;
use paddler_agent_embeddings::embedding_normalization::EmbeddingNormalization;
use paddler_inference_parameters::pooling_type::PoolingType;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_tests::active_embedding_request_fixture::ActiveEmbeddingRequestFixture;
use paddler_tests::nomic_embedding_batch_processing::NomicEmbeddingBatchProcessing;

#[test]
fn an_embedding_input_token_outside_the_vocabulary_fails_the_embedding_batch() {
    let nomic_embedding_batch_processing =
        NomicEmbeddingBatchProcessing::load(PoolingType::Mean, NonZeroU16::MIN)
            .expect("the nomic embedding model must load");
    let n_vocab = nomic_embedding_batch_processing
        .llama_runtime
        .loaded_llama_model
        .model
        .n_vocab();
    let ActiveEmbeddingRequestFixture {
        admitted_requests,
        generate_embedding_stop_tx: _generate_embedding_stop_tx,
        mut generated_embedding_rx,
    } = nomic_embedding_batch_processing.admit(vec![EmbeddingInputTokenized {
        id: "document".to_owned(),
        tokens: vec![LlamaToken::new(n_vocab)],
    }]);

    let continuing_requests = nomic_embedding_batch_processing
        .embed_next_batch(EmbeddingNormalization, admitted_requests)
        .expect("the embedding context must be created");

    assert_eq!(continuing_requests.len(), 0);
    assert_eq!(
        generated_embedding_rx.try_recv(),
        Ok(EmbeddingResult::BatchAssemblyFailed(
            EmbeddingError::BatchAssemblyFailed(BatchAddError::TokenOutsideVocabulary(
                TokenOutsideVocabulary {
                    token: LlamaToken::new(n_vocab),
                    n_vocab,
                }
            ))
            .to_string()
        ))
    );
}
