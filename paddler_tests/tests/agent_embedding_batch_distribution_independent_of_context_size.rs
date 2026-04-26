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
async fn agent_embedding_batch_distribution_independent_of_context_size() -> Result<()> {
    let cluster = start_in_process_embedding_cluster(
        InferenceParameters {
            batch_n_tokens: 64,
            context_size: 512,
            enable_embeddings: true,
            ..InferenceParameters::default()
        },
        4,
    )
    .await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let stream = inference_client
        .post_generate_embedding_batch(&GenerateEmbeddingBatchParams {
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

    let collected = collect_embedding_results(stream).await?;

    assert_eq!(collected.embeddings.len(), 4);
    assert!(collected.saw_done);
    assert!(collected.errors.is_empty());

    let returned_ids: BTreeSet<String> = collected
        .embeddings
        .iter()
        .map(|embedding| embedding.source_document_id.clone())
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
