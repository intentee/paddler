#![cfg(feature = "tests_that_use_llms")]

use std::collections::BTreeSet;

use anyhow::Result;
use paddler_cli_tests::qwen3_embedding_cluster_params::Qwen3EmbeddingClusterParams;
use paddler_cli_tests::start_subprocess_embedding_cluster::start_subprocess_embedding_cluster;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn balancer_distributes_embedding_batch_across_agents() -> Result<()> {
    let cluster = start_subprocess_embedding_cluster(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        Qwen3EmbeddingClusterParams {
            agents: AgentConfig::uniform(2, 4),
            inference_parameters: InferenceParameters {
                enable_embeddings: true,
                ..InferenceParameters::default()
            },
            ..Qwen3EmbeddingClusterParams::default()
        },
    )
    .await?;

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

    let collected = cluster.generate_embedding_batch(&params).await?;

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
