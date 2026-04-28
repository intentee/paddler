#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_tests::collect_embedding_results::collect_embedding_results;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::start_in_process_embedding_cluster::start_in_process_embedding_cluster;
use paddler_types::embedding_input_document::EmbeddingInputDocument;
use paddler_types::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::request_params::GenerateEmbeddingBatchParams;
use reqwest::Client;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn agent_embeddings_share_dimension_across_inputs_of_varying_length() -> Result<()> {
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

    let stream = inference_client
        .post_generate_embedding_batch(&GenerateEmbeddingBatchParams {
            input_batch: vec![
                EmbeddingInputDocument {
                    content: "Hello".to_owned(),
                    id: "doc-short".to_owned(),
                },
                EmbeddingInputDocument {
                    content: "The quick brown fox jumped over the lazy dog.".to_owned(),
                    id: "doc-medium".to_owned(),
                },
                EmbeddingInputDocument {
                    content: "Rust is a systems programming language focused on safety, speed, and concurrency. It achieves memory safety without garbage collection.".to_owned(),
                    id: "doc-long".to_owned(),
                },
            ],
            normalization_method: EmbeddingNormalizationMethod::None,
        })
        .await?;

    let collected = collect_embedding_results(stream).await?;

    assert_eq!(collected.embeddings.len(), 3);
    assert!(collected.saw_done);

    let first_dimension = collected.embeddings[0].embedding.len();

    assert!(first_dimension > 0, "embedding dimension must be positive");

    for embedding in &collected.embeddings {
        assert_eq!(
            embedding.embedding.len(),
            first_dimension,
            "all embeddings must share dimension; {} has {} instead of {}",
            embedding.source_document_id,
            embedding.embedding.len(),
            first_dimension
        );
    }

    cluster.shutdown().await?;

    Ok(())
}
