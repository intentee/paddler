#![cfg(feature = "tests_that_use_llms")]

use std::collections::BTreeSet;

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
async fn agent_chunks_embedding_batch_larger_than_slot_count() -> Result<()> {
    let cluster = start_in_process_embedding_cluster(
        InferenceParameters {
            enable_embeddings: true,
            ..InferenceParameters::default()
        },
        4,
    )
    .await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let input_batch: Vec<EmbeddingInputDocument> = (0..12)
        .map(|index| EmbeddingInputDocument {
            content: format!("Document number {index}."),
            id: format!("doc-{index}"),
        })
        .collect();

    let stream = inference_client
        .post_generate_embedding_batch(&GenerateEmbeddingBatchParams {
            input_batch,
            normalization_method: EmbeddingNormalizationMethod::None,
        })
        .await?;

    let collected = collect_embedding_results(stream).await?;

    assert_eq!(collected.embeddings.len(), 12);
    assert!(collected.saw_done);
    assert!(collected.errors.is_empty());

    let returned_ids: BTreeSet<String> = collected
        .embeddings
        .iter()
        .map(|embedding| embedding.source_document_id.clone())
        .collect();
    let expected_ids: BTreeSet<String> = (0..12).map(|index| format!("doc-{index}")).collect();

    assert_eq!(returned_ids, expected_ids);

    let first_dimension = collected.embeddings[0].embedding.len();

    for embedding in &collected.embeddings {
        assert_eq!(embedding.embedding.len(), first_dimension);
    }

    cluster.shutdown().await?;

    Ok(())
}
