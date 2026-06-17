#![cfg(feature = "tests_that_use_llms")]

use std::collections::BTreeSet;

use std::time::Duration;

use anyhow::Result;
use futures_util::future;
use paddler_cli_tests::start_subprocess_embedding_cluster::start_subprocess_embedding_cluster;
use paddler_cluster::agent_config::AgentConfig;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_model_card::qwen3_embedding_cluster_params::Qwen3EmbeddingClusterParams;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_distributes_embedding_burst_evenly_across_agents() -> Result<()> {
    const AGENT_COUNT: usize = 4;
    const SLOTS_PER_AGENT: i32 = 2;
    const CONCURRENT_REQUESTS: usize = 8;

    let cluster = start_subprocess_embedding_cluster(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        Qwen3EmbeddingClusterParams {
            agents: AgentConfig::uniform(AGENT_COUNT, SLOTS_PER_AGENT),
            buffered_request_timeout: Duration::from_mins(1),
            inference_parameters: InferenceParameters {
                enable_embeddings: true,
                ..InferenceParameters::default()
            },
            max_buffered_requests: 32,
        },
    )
    .await?;

    let request_params: Vec<GenerateEmbeddingBatchParams> = (0..CONCURRENT_REQUESTS)
        .map(|request_index| {
            let input_batch: Vec<EmbeddingInputDocument> = (0..4)
                .map(|document_index| EmbeddingInputDocument {
                    content: format!(
                        "Burst request {request_index}, document {document_index}: \
                         provide an embedding for evaluation."
                    ),
                    id: format!("req-{request_index}-doc-{document_index}"),
                })
                .collect();

            GenerateEmbeddingBatchParams {
                input_batch,
                normalization_method: EmbeddingNormalizationMethod::None,
            }
        })
        .collect();

    let collection_futures = request_params.iter().map(|params| {
        cluster
            .inference_client
            .http()
            .generate_embedding_batch_collected(params)
    });

    let collected_streams = future::try_join_all(collection_futures).await?;

    let producers_across_streams: BTreeSet<&str> = collected_streams
        .iter()
        .flat_map(|collected| collected.embeddings.iter())
        .filter_map(|produced| produced.generated_by.as_deref())
        .collect();

    assert_eq!(
        producers_across_streams.len(),
        AGENT_COUNT,
        "burst of {CONCURRENT_REQUESTS} embedding batches across {AGENT_COUNT} agents must reach every agent, but saw producers: {producers_across_streams:?}",
    );

    for collected in &collected_streams {
        assert!(collected.saw_done);
        assert!(collected.errors.is_empty());
        assert_eq!(collected.embeddings.len(), 4);
    }

    cluster.shutdown().await?;

    Ok(())
}
