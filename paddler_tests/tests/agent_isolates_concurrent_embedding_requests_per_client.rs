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
async fn agent_isolates_concurrent_embedding_requests_per_client() -> Result<()> {
    let client_count: usize = 4;
    let docs_per_client: usize = 3;

    let cluster = start_embedding_cluster(Qwen3EmbeddingClusterParams {
        agents: vec![AgentConfig::single(4)],
        inference_parameters: InferenceParameters {
            enable_embeddings: true,
            ..InferenceParameters::default()
        },
        ..Qwen3EmbeddingClusterParams::default()
    })
    .await?;

    let client_tasks = (0..client_count).map(|client_index| {
        let input_batch: Vec<EmbeddingInputDocument> = (0..docs_per_client)
            .map(|document_index| EmbeddingInputDocument {
                content: format!("Content from client {client_index} document {document_index}."),
                id: format!("client-{client_index}-doc-{document_index}"),
            })
            .collect();

        cluster.generate_embedding_batch(&GenerateEmbeddingBatchParams {
            input_batch,
            normalization_method: EmbeddingNormalizationMethod::None,
        })
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
            .map(|produced| produced.embedding.source_document_id.clone())
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
