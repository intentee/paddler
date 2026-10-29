#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
use paddler_inference_parameters::pooling_type::PoolingType;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::numbered_embedding_documents::numbered_embedding_documents;
use paddler_tests::qwen3_desired_state::qwen3_desired_state;
use paddler_tests::start_cluster::start_cluster;

use crate::slot_held_by_an_unending_generation::SlotHeldByAnUnendingGeneration;

const SLOT_COUNT: u16 = 2;

#[tokio::test(flavor = "multi_thread")]
async fn an_embedding_batch_runs_on_the_slots_its_agent_has_free() {
    let mut cluster = start_cluster(ClusterParams {
        agents: vec![AgentConfig::single(SLOT_COUNT)],
        desired_state: ClusterDesiredState::Apply(Box::new(BalancerDesiredState {
            embeddings: EmbeddingParameters {
                pooling_type: PoolingType::Last,
                ..EmbeddingParameters::default()
            },
            ..qwen3_desired_state()
        })),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");
    let slot_held_by_an_unending_generation =
        SlotHeldByAnUnendingGeneration::hold(&mut cluster).await;
    let embedded = cluster
        .generate_embedding_batch(
            CancellationToken::new(),
            &GenerateEmbeddingBatchParams {
                input_batch: numbered_embedding_documents(usize::from(SLOT_COUNT)),
                normalization_method: EmbeddingNormalizationMethod::None,
            },
        )
        .await
        .expect("the embedding batch must be accepted");

    assert_eq!(
        embedded
            .embeddings
            .iter()
            .map(|embedding_with_producer| embedding_with_producer
                .embedding
                .source_document_id
                .clone())
            .collect::<Vec<_>>(),
        vec!["doc-0".to_owned(), "doc-1".to_owned()]
    );

    slot_held_by_an_unending_generation.release().await;

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
