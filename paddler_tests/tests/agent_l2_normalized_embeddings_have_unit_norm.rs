#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_cluster::agent_config::AgentConfig;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_model_card::qwen3_embedding_cluster_params::Qwen3EmbeddingClusterParams;
use paddler_tests::start_embedding_cluster::start_embedding_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn agent_l2_normalized_embeddings_have_unit_norm() -> Result<()> {
    let cluster = start_embedding_cluster(Qwen3EmbeddingClusterParams {
        agents: vec![AgentConfig::single(1)],
        inference_parameters: InferenceParameters {
            enable_embeddings: true,
            ..InferenceParameters::default()
        },
        ..Qwen3EmbeddingClusterParams::default()
    })
    .await?;

    let collected = cluster
        .inference_client
        .http()
        .generate_embedding_batch_collected(&GenerateEmbeddingBatchParams {
            input_batch: vec![EmbeddingInputDocument {
                content: "Testing L2 normalization on embeddings".to_owned(),
                id: "doc-l2".to_owned(),
            }],
            normalization_method: EmbeddingNormalizationMethod::L2,
        })
        .await?;

    assert_eq!(collected.embeddings.len(), 1);
    assert!(collected.saw_done);

    let produced = &collected.embeddings[0];

    assert!(matches!(
        produced.embedding.normalization_method,
        EmbeddingNormalizationMethod::L2
    ));

    let l2_norm: f32 = produced
        .embedding
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
