#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use std::collections::BTreeSet;

use anyhow::Result;
use paddler_tests::collect_embedding_results::collect_embedding_results;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::start_subprocess_cluster_with_qwen3_embedding::start_subprocess_cluster_with_qwen3_embedding;
use paddler_types::embedding_input_document::EmbeddingInputDocument;
use paddler_types::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::request_params::GenerateEmbeddingBatchParams;
use reqwest::Client;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn balancer_distributes_embedding_batch_across_agents() -> Result<()> {
    let cluster = start_subprocess_cluster_with_qwen3_embedding(
        InferenceParameters {
            enable_embeddings: true,
            ..InferenceParameters::default()
        },
        4,
        2,
    )
    .await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let filler = "x".repeat(380);
    let input_batch: Vec<EmbeddingInputDocument> = (0..12)
        .map(|index| EmbeddingInputDocument {
            content: format!("Document number {index:02}: {filler}"),
            id: format!("doc-{index}"),
        })
        .collect();
    let params = GenerateEmbeddingBatchParams {
        input_batch,
        normalization_method: EmbeddingNormalizationMethod::None,
    };

    let stream = inference_client
        .post_generate_embedding_batch(&params)
        .await?;
    let collected = collect_embedding_results(stream).await?;

    assert_eq!(collected.embeddings.len(), 12);
    assert!(collected.saw_done);
    assert!(collected.errors.is_empty());

    let producers: BTreeSet<&str> = collected
        .embeddings
        .iter()
        .filter_map(|produced| produced.generated_by.as_deref())
        .collect();

    assert!(
        producers.len() >= 2,
        "expected the embedding batch to be distributed across at least two agents, but only saw producers: {producers:?}"
    );

    cluster.shutdown().await?;

    Ok(())
}
