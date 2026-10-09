#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroUsize;

use tokio_util::sync::CancellationToken;

use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::embedding_cluster_params::EmbeddingClusterParams;
use paddler_tests::start_embedding_cluster::start_embedding_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_reports_one_request_id_across_a_chunked_embedding_batch() {
    let cluster = start_embedding_cluster(EmbeddingClusterParams {
        agents: vec![AgentConfig::single(2)],
        embedding_parameters: EmbeddingParameters {
            embedding_batch_size: NonZeroUsize::MIN,
            ..EmbeddingParameters::default()
        },
        ..EmbeddingClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let collected = cluster
        .generate_embedding_batch(
            CancellationToken::new(),
            &GenerateEmbeddingBatchParams {
                input_batch: (0..2)
                    .map(|index| EmbeddingInputDocument {
                        content: format!("Document number {index}."),
                        id: format!("doc-{index}"),
                    })
                    .collect(),
                normalization_method: EmbeddingNormalizationMethod::None,
            },
        )
        .await
        .expect("the embedding batch must be accepted");

    assert_eq!(collected.embeddings.len(), 2);
    assert!(collected.saw_done);
    assert_eq!(collected.request_ids.len(), 1);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
