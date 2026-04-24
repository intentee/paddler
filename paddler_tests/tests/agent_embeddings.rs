#![cfg(feature = "tests_that_use_llms")]

use std::collections::BTreeSet;

use anyhow::Result;
use paddler_tests::collect_embedding_results::collect_embedding_results;
use paddler_tests::in_process_cluster::InProcessCluster;
use paddler_tests::in_process_cluster_params::InProcessClusterParams;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_embedding_0_6b::qwen3_embedding_0_6b;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::embedding_input_document::EmbeddingInputDocument;
use paddler_types::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::request_params::GenerateEmbeddingBatchParams;
use reqwest::Client;

#[tokio::test(flavor = "multi_thread")]
async fn embedding_batch_returns_one_embedding_per_input_document() -> Result<()> {
    let ModelCard { reference, .. } = qwen3_embedding_0_6b();

    let desired_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters {
            enable_embeddings: true,
            ..InferenceParameters::default()
        },
        model: AgentDesiredModel::HuggingFace(reference),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: false,
    };

    let cluster = InProcessCluster::start(InProcessClusterParams {
        agent_count: 1,
        slots_per_agent: 1,
        desired_state,
        wait_for_slots_ready: true,
        ..InProcessClusterParams::default()
    })
    .await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let stream = inference_client
        .post_generate_embedding_batch(&GenerateEmbeddingBatchParams {
            input_batch: vec![
                EmbeddingInputDocument {
                    content: "The quick brown fox jumps over the lazy dog".to_owned(),
                    id: "doc-alpha".to_owned(),
                },
                EmbeddingInputDocument {
                    content: "Machine learning is a subset of artificial intelligence".to_owned(),
                    id: "doc-beta".to_owned(),
                },
            ],
            normalization_method: EmbeddingNormalizationMethod::None,
        })
        .await?;

    let collected = collect_embedding_results(stream).await?;

    assert_eq!(collected.embeddings.len(), 2);
    assert!(collected.saw_done);
    assert!(collected.errors.is_empty());

    let returned_ids: BTreeSet<String> = collected
        .embeddings
        .iter()
        .map(|embedding| embedding.source_document_id.clone())
        .collect();

    let expected_ids: BTreeSet<String> =
        BTreeSet::from(["doc-alpha".to_owned(), "doc-beta".to_owned()]);

    assert_eq!(returned_ids, expected_ids);

    cluster.shutdown().await?;

    Ok(())
}
