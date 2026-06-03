#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::request_params::GenerateEmbeddingBatchParams;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::qwen3_embedding_cluster_params::Qwen3EmbeddingClusterParams;
use paddler_tests::start_embedding_cluster::start_embedding_cluster;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn agent_returns_identical_embeddings_for_identical_documents() -> Result<()> {
    let cluster = start_embedding_cluster(Qwen3EmbeddingClusterParams {
        agents: vec![AgentConfig::single(1)],
        inference_parameters: InferenceParameters {
            enable_embeddings: true,
            ..InferenceParameters::default()
        },
        ..Qwen3EmbeddingClusterParams::default()
    })
    .await?;

    let repeated_content = "Deterministic embedding output test.";

    let collected = cluster
        .generate_embedding_batch(&GenerateEmbeddingBatchParams {
            input_batch: vec![
                EmbeddingInputDocument {
                    content: repeated_content.to_owned(),
                    id: "doc-first".to_owned(),
                },
                EmbeddingInputDocument {
                    content: repeated_content.to_owned(),
                    id: "doc-second".to_owned(),
                },
            ],
            normalization_method: EmbeddingNormalizationMethod::None,
        })
        .await?;

    assert_eq!(collected.embeddings.len(), 2);
    assert!(collected.saw_done);

    let first = collected
        .embeddings
        .iter()
        .find(|produced| produced.embedding.source_document_id == "doc-first")
        .context("first embedding missing")?;

    let second = collected
        .embeddings
        .iter()
        .find(|produced| produced.embedding.source_document_id == "doc-second")
        .context("second embedding missing")?;

    assert_eq!(
        first.embedding.embedding, second.embedding.embedding,
        "identical documents must produce identical embedding vectors"
    );

    cluster.shutdown().await?;

    Ok(())
}
