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

#[tokio::test(flavor = "multi_thread")]
async fn agent_isolates_concurrent_embedding_requests_per_client() -> Result<()> {
    let client_count: usize = 4;
    let docs_per_client: usize = 3;

    let cluster = start_in_process_embedding_cluster(
        InferenceParameters {
            enable_embeddings: true,
            ..InferenceParameters::default()
        },
        4,
    )
    .await?;

    let inference_base_url = cluster.addresses.inference_base_url()?;

    let client_tasks = (0..client_count).map(|client_index| {
        let inference_base_url = inference_base_url.clone();

        async move {
            let inference_client = InferenceHttpClient::new(Client::new(), inference_base_url);

            let input_batch: Vec<EmbeddingInputDocument> = (0..docs_per_client)
                .map(|document_index| EmbeddingInputDocument {
                    content: format!(
                        "Content from client {client_index} document {document_index}."
                    ),
                    id: format!("client-{client_index}-doc-{document_index}"),
                })
                .collect();

            let stream = inference_client
                .post_generate_embedding_batch(&GenerateEmbeddingBatchParams {
                    input_batch,
                    normalization_method: EmbeddingNormalizationMethod::None,
                })
                .await?;

            collect_embedding_results(stream).await
        }
    });

    let per_client_results = futures_util::future::join_all(client_tasks).await;

    assert_eq!(per_client_results.len(), client_count);

    for (client_index, embeddings) in per_client_results.into_iter().enumerate() {
        let collected = embeddings?;

        assert_eq!(
            collected.embeddings.len(),
            docs_per_client,
            "client {client_index} should receive all its embeddings"
        );

        let returned_ids: BTreeSet<String> = collected
            .embeddings
            .iter()
            .map(|embedding| embedding.source_document_id.clone())
            .collect();
        let expected_ids: BTreeSet<String> = (0..docs_per_client)
            .map(|document_index| format!("client-{client_index}-doc-{document_index}"))
            .collect();

        assert_eq!(
            returned_ids, expected_ids,
            "client {client_index} should receive exactly its own document ids"
        );
    }

    cluster.shutdown().await?;

    Ok(())
}
