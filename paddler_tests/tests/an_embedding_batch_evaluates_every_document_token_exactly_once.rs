#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU16;

use llama_cpp_bindings::token::LlamaToken;

use paddler_agent_embeddings::embedding_input_tokenized::EmbeddingInputTokenized;
use paddler_agent_embeddings::embedding_normalization::EmbeddingNormalization;
use paddler_inference_parameters::pooling_type::PoolingType;
use paddler_tests::active_embedding_request_fixture::ActiveEmbeddingRequestFixture;

use crate::nomic_embedding_batch_processing::NomicEmbeddingBatchProcessing;

const REQUEST_SEQUENCES: NonZeroU16 = NonZeroU16::new(2).unwrap();

#[test]
fn an_embedding_batch_evaluates_every_document_token_exactly_once() {
    let nomic_embedding_batch_processing =
        NomicEmbeddingBatchProcessing::load(PoolingType::Mean, REQUEST_SEQUENCES);
    let documents = vec![
        EmbeddingInputTokenized {
            id: "short".to_owned(),
            tokens: vec![
                LlamaToken::new(101),
                LlamaToken::new(2023),
                LlamaToken::new(102),
            ],
        },
        EmbeddingInputTokenized {
            id: "long".to_owned(),
            tokens: vec![
                LlamaToken::new(101),
                LlamaToken::new(2023),
                LlamaToken::new(2003),
                LlamaToken::new(1037),
                LlamaToken::new(102),
            ],
        },
    ];
    let document_tokens: usize = documents.iter().map(|document| document.tokens.len()).sum();
    let ActiveEmbeddingRequestFixture {
        mut admitted_requests,
        generate_embedding_stop_tx: _generate_embedding_stop_tx,
        generated_embedding_rx: _generated_embedding_rx,
    } = nomic_embedding_batch_processing.admit(documents);
    let mut llama_context = nomic_embedding_batch_processing.embedding_llama_context();

    llama_context.reset_timings();

    while !admitted_requests.is_empty() {
        admitted_requests = nomic_embedding_batch_processing.embed_next_batch_in(
            &mut llama_context,
            nomic_embedding_batch_processing.batch_capacity(),
            EmbeddingNormalization,
            admitted_requests,
        );
    }

    llama_context.synchronize();

    assert_eq!(
        usize::try_from(llama_context.timings().n_p_eval())
            .expect("the evaluated token count must not be negative"),
        document_tokens
    );
}
