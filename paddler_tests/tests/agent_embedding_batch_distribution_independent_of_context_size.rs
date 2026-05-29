#![cfg(feature = "tests_that_use_llms")]

use std::collections::BTreeSet;

use anyhow::Result;
use paddler::embedding_input_document::EmbeddingInputDocument;
use paddler::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler::inference_parameters::InferenceParameters;
use paddler::request_params::GenerateEmbeddingBatchParams;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::qwen3_embedding_cluster_params::Qwen3EmbeddingClusterParams;
use paddler_tests::start_embedding_cluster::start_embedding_cluster;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn agent_embedding_batch_distribution_independent_of_context_size() -> Result<()> {
    let cluster = start_embedding_cluster(Qwen3EmbeddingClusterParams {
        agents: vec![AgentConfig::single(4)],
        inference_parameters: InferenceParameters {
            n_batch: 64,
            context_size: 512,
            enable_embeddings: true,
            ..InferenceParameters::default()
        },
        ..Qwen3EmbeddingClusterParams::default()
    })
    .await?;

    let collected = cluster
        .generate_embedding_batch(&GenerateEmbeddingBatchParams {
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
