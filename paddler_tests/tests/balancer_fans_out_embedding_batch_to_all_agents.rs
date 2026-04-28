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
async fn balancer_fans_out_embedding_batch_to_all_agents() -> Result<()> {
    let agent_count: usize = 4;

    let mut cluster = start_subprocess_cluster_with_qwen3_embedding(
        InferenceParameters {
            enable_embeddings: true,
            ..InferenceParameters::default()
        },
        2,
        agent_count,
    )
    .await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let filler = "x".repeat(380);
    let input_batch: Vec<EmbeddingInputDocument> = (0..16)
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

        seen_busy_agents.len() >= agent_count || (have_seen_any_activity && !any_busy_now)
    });

    let (request_result, observation_result) = tokio::join!(request_future, observation_future);
    let collected = request_result?;
    observation_result?;

    assert_eq!(collected.embeddings.len(), 16);
    assert!(collected.saw_done);
    assert!(collected.errors.is_empty());
    assert_eq!(
        seen_busy_agents.len(),
        agent_count,
        "expected the embedding batch to fan out across every agent, but only saw activity on: {seen_busy_agents:?}"
    );

    cluster.shutdown().await?;

    Ok(())
}
