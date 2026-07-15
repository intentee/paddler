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
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_distributes_embedding_batch_across_agents_with_uneven_slots() -> Result<()> {
    let cluster = start_subprocess_embedding_cluster(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        Qwen3EmbeddingClusterParams {
            agents: vec![
                AgentConfig {
                    name: "agent-fat".to_owned(),
                    slot_count: 4,
                },
                AgentConfig {
                    name: "agent-thin-a".to_owned(),
                    slot_count: 1,
                },
                AgentConfig {
                    name: "agent-medium".to_owned(),
                    slot_count: 2,
                },
                AgentConfig {
                    name: "agent-thin-b".to_owned(),
                    slot_count: 1,
                },
            ],
            inference_parameters: InferenceParameters {
                enable_embeddings: true,
                ..InferenceParameters::default()
            },
            ..Qwen3EmbeddingClusterParams::default()
        },
    )
    .await?;

    let input_batch: Vec<EmbeddingInputDocument> = (0..8)
        .map(|index| EmbeddingInputDocument {
            content: format!("Uneven-slot document number {index}."),
            id: format!("doc-{index}"),
        })
        .collect();

    let collected = cluster
        .generate_embedding_batch(
            CancellationToken::new(),
            &GenerateEmbeddingBatchParams {
                input_batch,
                normalization_method: EmbeddingNormalizationMethod::None,
            },
        )
        .await?;

    assert_eq!(collected.embeddings.len(), 8);
    assert!(collected.saw_done);
    assert!(collected.errors.is_empty());

    let returned_document_ids: BTreeSet<String> = collected
        .embeddings
        .iter()
        .map(|produced| produced.embedding.source_document_id.clone())
        .collect();
    let expected_document_ids: BTreeSet<String> =
        (0..8).map(|index| format!("doc-{index}")).collect();
    assert_eq!(returned_document_ids, expected_document_ids);

    let producers: BTreeSet<&str> = collected
        .embeddings
        .iter()
        .filter_map(|produced| produced.generated_by.as_deref())
        .collect();

    assert_eq!(
        producers.len(),
        4,
        "embedding batch must fan out across all agents even when slot counts are uneven, but only saw producers: {producers:?}",
    );

    cluster.shutdown().await?;

    Ok(())
}
