#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::nomic_embed_text_v1_5::nomic_embed_text_v1_5;
use paddler_tests::start_cluster::start_cluster;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn agent_serves_embeddings_without_a_chat_template() -> Result<()> {
    let ModelCard { reference, .. } = nomic_embed_text_v1_5();

    let cluster = start_cluster(ClusterParams {
        agents: AgentConfig::uniform(1, 1),
        wait_for_slots_ready: true,
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters {
                enable_embeddings: true,
                ..InferenceParameters::default()
            },
            model: AgentDesiredModel::HuggingFace(reference),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: false,
        }),
        ..ClusterParams::default()
    })
    .await?;

    let collected = cluster
        .generate_embedding_batch(
            CancellationToken::new(),
            &GenerateEmbeddingBatchParams {
                input_batch: vec![EmbeddingInputDocument {
                    content: "the quick brown fox jumps over the lazy dog".to_owned(),
                    id: "doc-1".to_owned(),
                }],
                normalization_method: EmbeddingNormalizationMethod::None,
            },
        )
        .await?;

    assert_eq!(collected.embeddings.len(), 1);
    assert!(collected.saw_done);
    assert!(collected.errors.is_empty());

    cluster.shutdown().await?;

    Ok(())
}
