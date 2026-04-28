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

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn balancer_distributes_embedding_batch_across_agents() -> Result<()> {
    let mut cluster = start_in_process_embedding_cluster(
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

    let mut seen_busy_agents: BTreeSet<String> = BTreeSet::new();
    let mut have_seen_any_activity = false;

    let request_future = async {
        let stream = inference_client
            .post_generate_embedding_batch(&params)
            .await?;
        collect_embedding_results(stream).await
    };

    let observation_future = cluster.agents.until(|snapshot| {
        let any_busy_now = snapshot
            .agents
            .iter()
            .any(|agent| agent.slots_processing > 0);

        if any_busy_now {
            have_seen_any_activity = true;
            for agent in &snapshot.agents {
                if agent.slots_processing > 0 {
                    seen_busy_agents.insert(agent.id.clone());
                }
            }
        }

        seen_busy_agents.len() >= 2 || (have_seen_any_activity && !any_busy_now)
    });

    let (request_result, observation_result) = tokio::join!(request_future, observation_future);
    let collected = request_result?;
    observation_result?;

    assert_eq!(collected.embeddings.len(), 12);
    assert!(collected.saw_done);
    assert!(collected.errors.is_empty());
    assert!(
        seen_busy_agents.len() >= 2,
        "expected the embedding batch to be distributed across at least two agents, but only saw activity on: {seen_busy_agents:?}"
    );

    cluster.shutdown().await?;

    Ok(())
}
