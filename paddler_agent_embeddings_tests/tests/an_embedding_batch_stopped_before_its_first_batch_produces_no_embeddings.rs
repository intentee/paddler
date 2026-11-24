#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU16;

use llama_cpp_model::llama_token::LlamaToken;
use tokio::sync::mpsc::error::TryRecvError;

use paddler_agent_embeddings::embedding_input_tokenized::EmbeddingInputTokenized;
use paddler_agent_embeddings_tests::admitted_embedding_requests::AdmittedEmbeddingRequests;
use paddler_agent_embeddings_tests::nomic_embedding_batch_processing::NomicEmbeddingBatchProcessing;
use paddler_inference_parameters::pooling_type::PoolingType;
use paddler_messaging::embedding_result::EmbeddingResult;

#[test]
fn an_embedding_batch_stopped_before_its_first_batch_produces_no_embeddings() {
    let nomic_embedding_batch_processing =
        NomicEmbeddingBatchProcessing::load(PoolingType::Mean, NonZeroU16::MIN)
            .expect("the nomic embedding model must load");
    let AdmittedEmbeddingRequests {
        admitted_requests,
        generate_embedding_stop_tx,
        mut generated_embedding_rx,
    } = nomic_embedding_batch_processing.admit(vec![EmbeddingInputTokenized {
        id: "document".to_owned(),
        tokens: vec![LlamaToken::new(0)],
    }]);

    generate_embedding_stop_tx
        .send(())
        .expect("the stop request must reach the embedding batch");

    let continuing_requests = nomic_embedding_batch_processing
        .embed_next_batch(admitted_requests)
        .expect("the embedding context must be created");

    assert_eq!(continuing_requests.len(), 0);
    assert_eq!(
        generated_embedding_rx.try_recv(),
        Ok(EmbeddingResult::NoEmbeddingsProduced)
    );
    assert_eq!(
        generated_embedding_rx.try_recv(),
        Err(TryRecvError::Disconnected)
    );
}
