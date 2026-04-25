#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use paddler_tests::collect_embedding_results::collect_embedding_results;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::start_in_process_embedding_cluster::start_in_process_embedding_cluster;
use paddler_types::embedding_input_document::EmbeddingInputDocument;
use paddler_types::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::request_params::GenerateEmbeddingBatchParams;
use reqwest::Client;

#[tokio::test(flavor = "multi_thread")]
async fn agent_returns_identical_embeddings_for_identical_documents() -> Result<()> {
    let cluster = start_in_process_embedding_cluster(
        InferenceParameters {
            enable_embeddings: true,
            ..InferenceParameters::default()
        },
        1,
    )
    .await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let repeated_content = "Deterministic embedding output test.";

    let stream = inference_client
        .post_generate_embedding_batch(&GenerateEmbeddingBatchParams {
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

    let collected = collect_embedding_results(stream).await?;

    assert_eq!(collected.embeddings.len(), 2);
    assert!(collected.saw_done);

    let first = collected
        .embeddings
        .iter()
        .find(|embedding| embedding.source_document_id == "doc-first")
        .context("first embedding missing")?;

    let second = collected
        .embeddings
        .iter()
        .find(|embedding| embedding.source_document_id == "doc-second")
        .context("second embedding missing")?;

    assert_eq!(
        first.embedding, second.embedding,
        "identical documents must produce identical embedding vectors"
    );

    cluster.shutdown().await?;

    Ok(())
}
