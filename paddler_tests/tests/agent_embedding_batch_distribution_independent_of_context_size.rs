#![cfg(feature = "tests_that_use_llms")]

use std::collections::BTreeSet;
use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_inference_parameters::batch_size::BatchSize;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::embedding_cluster_params::EmbeddingClusterParams;
use paddler_tests::start_embedding_cluster::start_embedding_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn agent_embedding_batch_distribution_independent_of_context_size() {
    let cluster = start_embedding_cluster(EmbeddingClusterParams {
        agents: vec![AgentConfig::single(4)],
        model_runtime_parameters: ModelRuntimeParameters {
            n_batch: BatchSize::try_from(64).expect("the value must fit its target type"),
            context_size: NonZeroU32::try_from(512).expect("the value must fit its target type"),
            ..ModelRuntimeParameters::default()
        },
        ..EmbeddingClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let collected = cluster
        .generate_embedding_batch(CancellationToken::new(), &GenerateEmbeddingBatchParams {
            input_batch: vec![
                EmbeddingInputDocument {
                    content: "This is the first document with enough content to contribute meaningfully to the batch size calculation".to_owned(),
                    id: "doc-chunk-1".to_owned(),
                },
                EmbeddingInputDocument {
                    content: "This is the second document that should be processed in a potentially different batch from the first".to_owned(),
                    id: "doc-chunk-2".to_owned(),
                },
                EmbeddingInputDocument {
                    content: "This is the third document adding more content to ensure the total exceeds the configured chunk limit".to_owned(),
                    id: "doc-chunk-3".to_owned(),
                },
                EmbeddingInputDocument {
                    content: "This is the fourth document which should demonstrate that batching distributes across agent requests".to_owned(),
                    id: "doc-chunk-4".to_owned(),
                },
            ],
            normalization_method: EmbeddingNormalizationMethod::None,
        })
        .await.expect("the embedding batch must be accepted");

    assert_eq!(collected.embeddings.len(), 4);
    assert!(collected.saw_done);
    assert!(collected.failures.is_empty());

    let returned_ids: BTreeSet<String> = collected
        .embeddings
        .iter()
        .map(|produced| produced.embedding.source_document_id.clone())
        .collect();

    let expected_ids: BTreeSet<String> = BTreeSet::from([
        "doc-chunk-1".to_owned(),
        "doc-chunk-2".to_owned(),
        "doc-chunk-3".to_owned(),
        "doc-chunk-4".to_owned(),
    ]);

    assert_eq!(returned_ids, expected_ids);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
