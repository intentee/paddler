#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use paddler_cluster::agent_config::AgentConfig;
use paddler_cluster::cluster::Cluster;
use paddler_cluster::cluster_params::ClusterParams;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_tests::in_process_cluster_backend::InProcessClusterBackend;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::nomic_embed_text_v1_5::nomic_embed_text_v1_5;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_reports_unable_to_find_chat_template_for_embedding_model() -> Result<()> {
    let ModelCard { reference, .. } = nomic_embed_text_v1_5();

    let mut cluster = Cluster::start(
        &InProcessClusterBackend::default(),
        ClusterParams {
            agents: AgentConfig::uniform(1, 1),
            wait_for_slots_ready: false,
            desired_state: Some(BalancerDesiredState {
                chat_template_override: None,
                inference_parameters: InferenceParameters::default(),
                model: AgentDesiredModel::HuggingFace(reference),
                multimodal_projection: AgentDesiredModel::None,
                use_chat_template_override: false,
            }),
        },
    )
    .await?;

    let agent_id = cluster
        .agents
        .first()
        .map(|agent| agent.id.clone())
        .context("cluster must have one registered agent")?;

    let predicate_agent_id = agent_id.clone();
    cluster
        .agents_watcher
        .until_agent(&agent_id, move |snapshot| {
            snapshot.agents.iter().any(|agent| {
                agent.id == predicate_agent_id
                    && agent
                        .issues
                        .iter()
                        .any(|issue| matches!(issue, AgentIssue::UnableToFindChatTemplate(_)))
            })
        })
        .await
        .context("balancer should report UnableToFindChatTemplate for embedding-only model")?;

    cluster.shutdown().await?;

    Ok(())
}
