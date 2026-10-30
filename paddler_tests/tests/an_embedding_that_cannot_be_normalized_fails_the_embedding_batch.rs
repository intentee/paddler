#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU16;

use llama_cpp_bindings::token::LlamaToken;
use tokio::sync::mpsc::error::TryRecvError;

use paddler_agent_embeddings::embedding_error::EmbeddingError;
use paddler_agent_embeddings::embedding_input_tokenized::EmbeddingInputTokenized;
use paddler_inference_parameters::pooling_type::PoolingType;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_tests::active_embedding_request_fixture::ActiveEmbeddingRequestFixture;
use paddler_tests::normalization_of_an_embedding_too_long_for_rms_norm::DIMENSIONS_TOO_MANY_FOR_RMS_NORM;
use paddler_tests::normalization_of_an_embedding_too_long_for_rms_norm::NormalizationOfAnEmbeddingTooLongForRmsNorm;

use crate::nomic_embedding_batch_processing::NomicEmbeddingBatchProcessing;

const REQUEST_SEQUENCES: NonZeroU16 = NonZeroU16::new(2).unwrap();

#[test]
fn an_embedding_that_cannot_be_normalized_fails_the_embedding_batch() {
    let nomic_embedding_batch_processing =
        NomicEmbeddingBatchProcessing::load(PoolingType::Mean, REQUEST_SEQUENCES);
    let ActiveEmbeddingRequestFixture {
        admitted_requests,
        generate_embedding_stop_tx: _generate_embedding_stop_tx,
        mut generated_embedding_rx,
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

    let continuing_requests = nomic_embedding_batch_processing.embed_next_batch(
        nomic_embedding_batch_processing.batch_capacity(),
        NormalizationOfAnEmbeddingTooLongForRmsNorm,
        admitted_requests,
    );

    assert_eq!(continuing_requests.len(), 0);
    assert_eq!(
        generated_embedding_rx.try_recv(),
        Ok(EmbeddingResult::EmbeddingTooLongForRmsNormalization(
            EmbeddingError::EmbeddingTooLongForRmsNormalization {
                dimensions: DIMENSIONS_TOO_MANY_FOR_RMS_NORM,
                source: u16::try_from(DIMENSIONS_TOO_MANY_FOR_RMS_NORM)
                    .expect_err("the dimensions must exceed sixteen bits"),
            }
            .to_string()
        ))
    );
    assert_eq!(
        generated_embedding_rx.try_recv(),
        Err(TryRecvError::Disconnected)
    );
}
