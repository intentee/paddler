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
async fn agent_l2_normalized_embeddings_have_unit_norm() -> Result<()> {
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
            input_batch: vec![EmbeddingInputDocument {
                content: "Testing L2 normalization on embeddings".to_owned(),
                id: "doc-l2".to_owned(),
            }],
            normalization_method: EmbeddingNormalizationMethod::L2,
        })
        .await?;

    let collected = collect_embedding_results(stream).await?;

    assert_eq!(collected.embeddings.len(), 1);
    assert!(collected.saw_done);

    let embedding = &collected.embeddings[0];

    assert!(matches!(
        embedding.normalization_method,
        EmbeddingNormalizationMethod::L2
    ));

    let l2_norm: f32 = embedding
        .embedding
        .iter()
        .map(|value| value * value)
        .sum::<f32>()
        .sqrt();

    assert!(
        (l2_norm - 1.0).abs() < 1e-4,
        "L2 norm should be approximately 1.0, got {l2_norm}"
    );

    cluster.shutdown().await?;

    Ok(())
}
