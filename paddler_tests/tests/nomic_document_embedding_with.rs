#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::embedding_vector::EmbeddingVector;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::nomic_embed_text_v1_5::nomic_embed_text_v1_5;
use paddler_tests::start_cluster::start_cluster;

pub async fn nomic_document_embedding_with(
    embedding_parameters: EmbeddingParameters,
) -> Vec<EmbeddingVector> {
    let cluster = start_cluster(ClusterParams {
        agents: AgentConfig::uniform(1, 1),
        desired_state: ClusterDesiredState::Apply(Box::new(
            nomic_embed_text_v1_5()
                .into_desired_state_with_embedding_parameters(embedding_parameters),
        )),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
    .expect("the nomic embedding cluster must start");

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
        .await
        .expect("the document must be embedded");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");

    collected
        .embeddings
        .into_iter()
        .map(|embedding_with_producer| embedding_with_producer.embedding.embedding)
        .collect()
}
