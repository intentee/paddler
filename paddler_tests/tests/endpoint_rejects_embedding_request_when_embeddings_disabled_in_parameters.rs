#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use paddler_client::error::Error as ClientError;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::start_cluster::start_cluster;

const NOT_IMPLEMENTED: u16 = 501;

#[tokio::test(flavor = "multi_thread")]
async fn endpoint_rejects_embedding_request_when_embeddings_disabled_in_parameters() -> Result<()> {
    let ModelCard { reference, .. } = qwen3_0_6b();

    let cluster = start_cluster(ClusterParams {
        agents: vec![AgentConfig {
            name: "test-agent".to_owned(),
            slot_count: 1,
        }],
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters::default(),
            model: AgentDesiredModel::HuggingFace(reference),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: false,
        }),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await?;

    let rejection = cluster
        .client_inference
        .post_generate_embedding_batch(&GenerateEmbeddingBatchParams {
            input_batch: vec![EmbeddingInputDocument {
                content: "Hello world".to_owned(),
                id: "doc-1".to_owned(),
            }],
            normalization_method: EmbeddingNormalizationMethod::None,
        })
        .await
        .err()
        .context("endpoint must reject embedding requests when embeddings are disabled")?;

    match rejection {
        ClientError::UnexpectedResponseStatus { status, .. } => assert_eq!(
            status.as_u16(),
            NOT_IMPLEMENTED,
            "endpoint must reject embedding requests with HTTP 501 when embeddings are disabled",
        ),
        other_error => panic!("expected an unexpected-status rejection, got: {other_error}"),
    }

    cluster.shutdown().await?;

    Ok(())
}
