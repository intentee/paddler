#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use paddler_cluster::agent_config::AgentConfig;
use paddler_cluster::cluster::Cluster;
use paddler_cluster::cluster_params::ClusterParams;
use paddler_cluster::desired_state_init::DesiredStateInit;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_model_card::model_card::ModelCard;
use paddler_model_card::nomic_embed_text_v1_5::nomic_embed_text_v1_5;
use paddler_tests::in_process_cluster_backend::InProcessClusterBackend;

#[tokio::test(flavor = "multi_thread")]
async fn chat_template_override_applied_to_embedding_model() -> Result<()> {
    let ModelCard { reference, .. } = nomic_embed_text_v1_5();

    let chat_template = ChatTemplate {
        content: "{{ messages[0].content }}".to_owned(),
    };

    let cluster = Cluster::start(
        &InProcessClusterBackend::default(),
        ClusterParams {
            agents: AgentConfig::uniform(1, 1),
            wait_for_slots_ready: false,
            desired_state: DesiredStateInit::set(BalancerDesiredState {
                chat_template_override: Some(chat_template.clone()),
                inference_parameters: InferenceParameters::default(),
                model: AgentDesiredModel::HuggingFace(reference),
                multimodal_projection: AgentDesiredModel::None,
                use_chat_template_override: true,
            }),
        },
    )
    .await?;

    let agent_id = cluster
        .agents
        .first()
        .map(|agent| agent.id.clone())
        .context("cluster must have one registered agent")?;

    let retrieved = cluster
        .management_client
        .chat_template_override(&agent_id)
        .await
        .map_err(anyhow::Error::new)
        .context("failed to read chat template override")?;

    assert_eq!(retrieved, Some(chat_template));

    cluster.shutdown().await?;

    Ok(())
}
