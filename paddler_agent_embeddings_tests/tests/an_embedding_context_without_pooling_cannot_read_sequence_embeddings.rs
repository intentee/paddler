#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU16;

use llama_cpp_model::llama_token::LlamaToken;

use paddler_agent_embeddings::embedding_input_tokenized::EmbeddingInputTokenized;
use paddler_agent_embeddings_tests::admitted_embedding_requests::AdmittedEmbeddingRequests;
use paddler_agent_embeddings_tests::nomic_embedding_batch_processing::NomicEmbeddingBatchProcessing;
use paddler_inference_parameters::pooling_type::PoolingType;
use paddler_messaging::embedding_result::EmbeddingResult;

#[test]
fn an_embedding_context_without_pooling_cannot_read_sequence_embeddings() {
    let nomic_embedding_batch_processing =
        NomicEmbeddingBatchProcessing::load(PoolingType::None, NonZeroU16::MIN)
            .expect("the nomic embedding model must load");
    let AdmittedEmbeddingRequests {
        admitted_requests,
        generate_embedding_stop_tx: _generate_embedding_stop_tx,
        mut generated_embedding_rx,
    } = nomic_embedding_batch_processing.admit(vec![EmbeddingInputTokenized {
        id: "document".to_owned(),
        tokens: vec![LlamaToken::new(101), LlamaToken::new(102)],
    }]);

    let continuing_requests = nomic_embedding_batch_processing
        .embed_next_batch(admitted_requests)
        .expect("the embedding context must be created");

    assert_eq!(continuing_requests.len(), 0);
    assert!(matches!(
        generated_embedding_rx.try_recv(),
        Ok(EmbeddingResult::EmbeddingsUnavailable(_))
    ));
}
