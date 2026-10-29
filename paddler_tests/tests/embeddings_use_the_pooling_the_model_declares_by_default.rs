#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroUsize;

use anyhow::Result;
use futures_util::TryFutureExt as _;
use tokio_util::sync::CancellationToken;

use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
use paddler_inference_parameters::pooling_type::PoolingType;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::embedding_vector::EmbeddingVector;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::nomic_embed_text_v1_5::nomic_embed_text_v1_5;
use paddler_tests::start_cluster::start_cluster;

async fn embedding_vectors_with(
    embedding_parameters: EmbeddingParameters,
) -> Result<Vec<EmbeddingVector>> {
    start_cluster(ClusterParams {
        agents: AgentConfig::uniform(1, 1),
        desired_state: ClusterDesiredState::Apply(Box::new(
            nomic_embed_text_v1_5()
                .into_desired_state_with_embedding_parameters(embedding_parameters),
        )),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .and_then(|cluster| async move {
        let collected = cluster
            .generate_embedding_batch(
                CancellationToken::new(),
                &GenerateEmbeddingBatchParams {
                    input_batch: vec![EmbeddingInputDocument {
                        content: "the quick brown fox jumps over the lazy dog".to_owned(),
                        id: "doc-1".to_owned(),
                    }],
                    normalization_method: EmbeddingNormalizationMethod::None,
                },
            )
            .await;

        cluster.shutdown().await.and(collected)
    })
    .map_ok(|collected| {
        collected
            .embeddings
            .into_iter()
            .map(|embedding_with_producer| embedding_with_producer.embedding.embedding)
            .collect()
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn embeddings_use_the_pooling_the_model_declares_by_default() {
    let default_embedding_parameters = EmbeddingParameters::default();
    let mean_pooled_embedding_parameters = EmbeddingParameters {
        embedding_batch_size: NonZeroUsize::MIN,
        pooling_type: PoolingType::Mean,
    };

    assert_eq!(
        embedding_vectors_with(EmbeddingParameters {
            embedding_batch_size: NonZeroUsize::MIN,
            ..default_embedding_parameters
        })
        .await
        .expect("the default pooling must embed the document"),
        embedding_vectors_with(mean_pooled_embedding_parameters)
            .await
            .expect("mean pooling must embed the document")
    );
}
