#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_client::error::Error;
use paddler_cluster::agent_config::AgentConfig;
use paddler_cluster::cluster::Cluster;
use paddler_cluster::cluster_params::ClusterParams;
use paddler_cluster::desired_state_init::DesiredStateInit;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_model_card::model_card::ModelCard;
use paddler_model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::in_process_cluster_backend::InProcessClusterBackend;

#[tokio::test(flavor = "multi_thread")]
async fn endpoint_rejects_embedding_request_when_embeddings_disabled_in_parameters() -> Result<()> {
    let ModelCard { reference, .. } = qwen3_0_6b();

    let cluster = Cluster::start(
        &InProcessClusterBackend::default(),
        ClusterParams {
            agents: vec![AgentConfig {
                name: "test-agent".to_owned(),
                slot_count: 1,
            }],
            desired_state: DesiredStateInit::set(BalancerDesiredState {
                chat_template_override: None,
                inference_parameters: InferenceParameters::default(),
                model: AgentDesiredModel::HuggingFace(reference),
                multimodal_projection: AgentDesiredModel::None,
                use_chat_template_override: false,
            }),
            wait_for_slots_ready: true,
        },
    )
    .await?;

    let outcome = cluster
        .inference_client
        .http()
        .generate_embedding_batch(&GenerateEmbeddingBatchParams {
            input_batch: vec![EmbeddingInputDocument {
                content: "Hello world".to_owned(),
                id: "doc-1".to_owned(),
            }],
            normalization_method: EmbeddingNormalizationMethod::None,
        })
        .await;

    let rejected_status = match outcome {
        Err(Error::Http(http_error)) => http_error.status(),
        _ => None,
    };

    assert_eq!(
        rejected_status.map(|status| status.as_u16()),
        Some(501),
        "endpoint must reject embedding requests with HTTP 501 when embeddings are disabled",
    );

    cluster.shutdown().await?;

    Ok(())
}
