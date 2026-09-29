#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use std::collections::BTreeSet;

use anyhow::Result;
use paddler_inference_parameters::batch_size::BatchSize;
use paddler_inference_parameters::inference_parameters::InferenceParameters;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::embedding_cluster_params::EmbeddingClusterParams;
use paddler_tests::start_embedding_cluster::start_embedding_cluster;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn agent_embedding_batch_distribution_independent_of_context_size() -> Result<()> {
    let cluster = start_embedding_cluster(EmbeddingClusterParams {
        agents: vec![AgentConfig::single(4)],
        inference_parameters: InferenceParameters {
            n_batch: BatchSize::try_from(64)?,
            context_size: NonZeroU32::try_from(512)?,
            enable_embeddings: true,
            ..InferenceParameters::default()
        },
        ..EmbeddingClusterParams::default()
    })
    .await?;

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
        .await?;

    assert_eq!(collected.embeddings.len(), 4);
    assert!(collected.saw_done);
    assert!(collected.errors.is_empty());

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

    cluster.shutdown().await?;

    Ok(())
}
