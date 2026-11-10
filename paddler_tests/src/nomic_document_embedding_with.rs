use anyhow::Context as _;
use anyhow::Result;
use futures_util::FutureExt as _;
use futures_util::TryFutureExt as _;
use tokio_util::sync::CancellationToken;

use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
use paddler_messaging::embedding::Embedding;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::embedding_vector::EmbeddingVector;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::collected_embedding_results::CollectedEmbeddingResults;
use paddler_test_cluster_harness::embedding_with_producer::EmbeddingWithProducer;
use paddler_test_cluster_harness::model_card::nomic_embed_text_v1_5::nomic_embed_text_v1_5;

use crate::start_cluster::start_cluster;

async fn embed_the_document(cluster: Cluster) -> Result<Vec<EmbeddingVector>> {
    cluster
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
        .map(|embedded| embedded.context("the document must be embedded"))
        .and_then(|CollectedEmbeddingResults { embeddings, .. }| {
            cluster
                .shutdown()
                .map(|shut_down| shut_down.context("the cluster must shut down cleanly"))
                .map_ok(|()| {
                    embeddings
                        .into_iter()
                        .map(
                            |EmbeddingWithProducer {
                                 embedding: Embedding { vector, .. },
                                 ..
                             }| vector,
                        )
                        .collect()
                })
        })
        .await
}

/// # Errors
///
/// Returns an error when the nomic embedding cluster cannot start, when the document is not embedded, and when the cluster does not shut down cleanly.
pub async fn nomic_document_embedding_with(
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
    .map(|started| started.context("the nomic embedding cluster must start"))
    .and_then(embed_the_document)
    .await
}
