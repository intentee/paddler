#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use paddler_cli_tests::agent_config::AgentConfig;
use paddler_cli_tests::model_card::ModelCard;
use paddler_cli_tests::model_card::nomic_embed_text_v1_5::nomic_embed_text_v1_5;
use paddler_cli_tests::start_subprocess_cluster::start_subprocess_cluster;
use paddler_cli_tests::subprocess_cluster_params::SubprocessClusterParams;
use paddler::agent_desired_model::AgentDesiredModel;
use paddler::agent_issue::AgentIssue;
use paddler::balancer_desired_state::BalancerDesiredState;
use paddler::inference_parameters::InferenceParameters;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn balancer_reports_unable_to_find_chat_template_for_embedding_model() -> Result<()> {
    let ModelCard { reference, .. } = nomic_embed_text_v1_5();

    let mut cluster = start_subprocess_cluster(SubprocessClusterParams {
        agents: AgentConfig::uniform(1, 1),
        wait_for_slots_ready: false,
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters::default(),
            model: AgentDesiredModel::HuggingFace(reference),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: false,
        }),
        ..SubprocessClusterParams::default()
    })
    .await?;

    let agent_id = cluster
        .agent_ids
        .first()
        .context("cluster must have one registered agent")?
        .clone();

    let predicate_agent_id = agent_id.clone();
    cluster
        .agents
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
