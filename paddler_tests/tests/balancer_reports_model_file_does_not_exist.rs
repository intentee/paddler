#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::start_cluster::start_cluster;
use paddler_tests::cluster_params::ClusterParams;
use paddler::agent_desired_model::AgentDesiredModel;
use paddler::agent_issue::AgentIssue;
use paddler::balancer_desired_state::BalancerDesiredState;
use paddler::inference_parameters::InferenceParameters;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn balancer_reports_model_file_does_not_exist() -> Result<()> {
    let mut cluster = start_cluster(ClusterParams {
        agents: AgentConfig::uniform(1, 1),
        wait_for_slots_ready: false,
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters::default(),
            model: AgentDesiredModel::LocalToAgent("/nonexistent/model.gguf".to_owned()),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: false,
        }),
        ..ClusterParams::default()
    })
    .await?;

    let agent_id = cluster
        .agent_ids
        .first()
        .context("cluster must have one registered agent")?
        .clone();

    let snapshot = cluster
        .agents
        .until(move |snapshot| {
            snapshot.agents.iter().any(|agent| {
                agent.id == agent_id
                    && agent
                        .issues
                        .iter()
                        .any(|issue| matches!(issue, AgentIssue::ModelFileDoesNotExist(_)))
            })
        })
        .await
        .context("balancer should report ModelFileDoesNotExist for nonexistent path")?;

    let saw_expected_path = snapshot.agents.iter().any(|agent| {
        agent.issues.iter().any(|issue| {
            matches!(issue, AgentIssue::ModelFileDoesNotExist(model_path)
                if model_path.model_path == "/nonexistent/model.gguf")
        })
    });

    assert!(
        saw_expected_path,
        "ModelFileDoesNotExist should reference the configured path"
    );

    cluster.shutdown().await?;

    Ok(())
}
