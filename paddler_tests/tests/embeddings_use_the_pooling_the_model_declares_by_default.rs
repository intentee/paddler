#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroUsize;

use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
use paddler_inference_parameters::pooling_type::PoolingType;

use crate::nomic_document_embedding_with::nomic_document_embedding_with;

#[tokio::test(flavor = "multi_thread")]
async fn embeddings_use_the_pooling_the_model_declares_by_default() {
    assert_eq!(
        nomic_document_embedding_with(EmbeddingParameters {
            embedding_batch_size: NonZeroUsize::MIN,
            ..EmbeddingParameters::default()
        })
        .await,
        nomic_document_embedding_with(EmbeddingParameters {
            embedding_batch_size: NonZeroUsize::MIN,
            pooling_type: PoolingType::Mean,
        })
        .await
    );
}
