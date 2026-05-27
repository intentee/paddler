#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use std::collections::BTreeSet;

use std::time::Duration;

use anyhow::Result;
use futures_util::future;
use paddler_cli_tests::agent_config::AgentConfig;
use paddler_cli_tests::collect_embedding_results::collect_embedding_results;
use paddler_cli_tests::inference_http_client::InferenceHttpClient;
use paddler_cli_tests::qwen3_embedding_cluster_params::Qwen3EmbeddingClusterParams;
use paddler_cli_tests::start_subprocess_cluster_with_qwen3_embedding::start_subprocess_cluster_with_qwen3_embedding;
use paddler_types::embedding_input_document::EmbeddingInputDocument;
use paddler_types::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::request_params::GenerateEmbeddingBatchParams;
use reqwest::Client;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn balancer_distributes_embedding_burst_evenly_across_agents() -> Result<()> {
    const AGENT_COUNT: usize = 4;
    const SLOTS_PER_AGENT: i32 = 2;
    const CONCURRENT_REQUESTS: usize = 8;

    let cluster = start_subprocess_cluster_with_qwen3_embedding(Qwen3EmbeddingClusterParams {
        agents: AgentConfig::uniform(AGENT_COUNT, SLOTS_PER_AGENT),
        buffered_request_timeout: Duration::from_secs(60),
        inference_parameters: InferenceParameters {
            enable_embeddings: true,
            ..InferenceParameters::default()
        },
        max_buffered_requests: 32,
    })
    .await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let collection_futures = (0..CONCURRENT_REQUESTS).map(|request_index| {
        let inference_client = inference_client.clone();
        async move {
            let input_batch: Vec<EmbeddingInputDocument> = (0..4)
                .map(|document_index| EmbeddingInputDocument {
                    content: format!(
                        "Burst request {request_index}, document {document_index}: \
                         provide an embedding for evaluation."
                    ),
                    id: format!("req-{request_index}-doc-{document_index}"),
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
