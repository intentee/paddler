#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU16;

use llama_cpp_model::llama_token::LlamaToken;

use paddler_agent_embeddings::embedding_input_tokenized::EmbeddingInputTokenized;
use paddler_agent_embeddings_tests::admitted_embedding_requests::AdmittedEmbeddingRequests;
use paddler_agent_embeddings_tests::nomic_embedding_batch_processing::NomicEmbeddingBatchProcessing;
use paddler_inference_parameters::pooling_type::PoolingType;

const REQUEST_SEQUENCES: NonZeroU16 = NonZeroU16::new(2).unwrap();

#[test]
fn an_embedding_request_whose_client_left_ends_before_its_remaining_documents_are_delivered() {
    let nomic_embedding_batch_processing =
        NomicEmbeddingBatchProcessing::load(PoolingType::Mean, REQUEST_SEQUENCES)
            .expect("the nomic embedding model must load");
    let AdmittedEmbeddingRequests {
        admitted_requests,
        generate_embedding_stop_tx: _generate_embedding_stop_tx,
        generated_embedding_rx,
    } = nomic_embedding_batch_processing.admit(vec![
        EmbeddingInputTokenized {
            id: "first".to_owned(),
            tokens: vec![LlamaToken::new(101), LlamaToken::new(102)],
        },
        EmbeddingInputTokenized {
            id: "second".to_owned(),
            tokens: vec![LlamaToken::new(101), LlamaToken::new(102)],
        },
    ]);

    drop(generated_embedding_rx);

    let continuing_requests = nomic_embedding_batch_processing
        .embed_next_batch(admitted_requests)
        .expect("the embedding context must be created");

    assert_eq!(continuing_requests.len(), 0);
}
