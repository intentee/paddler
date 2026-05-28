#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::collect_embedding_results::collect_embedding_results;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::start_embedding_cluster::start_embedding_cluster;
use paddler::embedding_input_document::EmbeddingInputDocument;
use paddler::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler::inference_parameters::InferenceParameters;
use paddler::request_params::GenerateEmbeddingBatchParams;
use reqwest::Client;
use paddler_tests::qwen3_embedding_cluster_params::Qwen3EmbeddingClusterParams;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn agent_returns_rms_normalized_embeddings_when_requested() -> Result<()> {
    let cluster = start_embedding_cluster(Qwen3EmbeddingClusterParams {
        agents: vec![AgentConfig::single(1)],
        inference_parameters: InferenceParameters {
            enable_embeddings: true,
            ..InferenceParameters::default()
        },
        ..Qwen3EmbeddingClusterParams::default()
    })
    .await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let stream = inference_client
        .post_generate_embedding_batch(&GenerateEmbeddingBatchParams {
            input_batch: vec![EmbeddingInputDocument {
                content: "Testing RMS normalization on embeddings".to_owned(),
                id: "doc-rms".to_owned(),
            }],
            normalization_method: EmbeddingNormalizationMethod::RmsNorm { epsilon: 1e-6 },
        })
        .await?;

    let collected = collect_embedding_results(stream).await?;

    assert_eq!(collected.embeddings.len(), 1);
    assert!(collected.saw_done);
    assert!(matches!(
        collected.embeddings[0].embedding.normalization_method,
        EmbeddingNormalizationMethod::RmsNorm { .. }
    ));

    cluster.shutdown().await?;

    Ok(())
}
