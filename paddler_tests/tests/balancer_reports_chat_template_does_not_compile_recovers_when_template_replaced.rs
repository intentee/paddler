#![cfg(feature = "tests_that_use_llms")]

use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use paddler_cluster::agent_config::AgentConfig;
use paddler_cluster::cluster::Cluster;
use paddler_cluster::cluster_params::ClusterParams;
use paddler_cluster::desired_state_init::DesiredStateInit;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_model_card::model_card::ModelCard;
use paddler_model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::in_process_cluster_backend::InProcessClusterBackend;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_reports_chat_template_does_not_compile_recovers_when_template_replaced()
-> Result<()> {
    let ModelCard { reference, .. } = qwen3_0_6b();

    let invalid_template = ChatTemplate {
        content: "{{invalid jinja template".to_owned(),
    };
    let valid_template = ChatTemplate {
        content: "{% for message in messages %}{{ message.content }}{% endfor %}".to_owned(),
    };

    let mut cluster = Cluster::start(
        &InProcessClusterBackend::default(),
        ClusterParams {
            agents: AgentConfig::uniform(1, 1),
            wait_for_slots_ready: false,
            desired_state: DesiredStateInit::set(BalancerDesiredState {
                chat_template_override: Some(invalid_template),
                inference_parameters: InferenceParameters::default(),
                model: AgentDesiredModel::HuggingFace(reference.clone()),
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

    let predicate_agent_id = agent_id.clone();
    cluster
        .agents_watcher
        .until(move |snapshot| {
            snapshot.agents.iter().any(|agent| {
                agent.id == predicate_agent_id
                    && agent
                        .issues
                        .iter()
                        .any(|issue| matches!(issue, AgentIssue::ChatTemplateDoesNotCompile(_)))
            })
        })
        .await
        .context("balancer should report ChatTemplateDoesNotCompile for invalid Jinja syntax")?;

    let recovered_state = BalancerDesiredState {
        chat_template_override: Some(valid_template),
        inference_parameters: InferenceParameters::default(),
        model: AgentDesiredModel::HuggingFace(reference),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: true,
    };

    cluster
        .management_client
        .set_desired_state(&recovered_state)
        .await
        .map_err(anyhow::Error::new)
        .context("balancer should accept the recovered desired state")?;

    let predicate_agent_id_for_recovery = agent_id;
    tokio::time::timeout(
        Duration::from_secs(3),
        cluster.agents_watcher.until(move |snapshot| {
            snapshot.agents.iter().any(|agent| {
                agent.id == predicate_agent_id_for_recovery
                    && agent
                        .issues
                        .iter()
                        .all(|issue| !matches!(issue, AgentIssue::ChatTemplateDoesNotCompile(_)))
            })
        }),
    )
    .await
    .context("reconciliation should clear ChatTemplateDoesNotCompile within 3 seconds")??;

    cluster.shutdown().await?;

    Ok(())
}
