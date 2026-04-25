#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use anyhow::Context as _;
use anyhow::Result;
use paddler_tests::subprocess_cluster::SubprocessCluster;
use paddler_tests::subprocess_cluster_params::SubprocessClusterParams;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::agent_issue::AgentIssue;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::inference_parameters::InferenceParameters;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_reports_model_file_does_not_exist() -> Result<()> {
    let mut cluster = SubprocessCluster::start(SubprocessClusterParams {
        agent_count: 1,
        slots_per_agent: 1,
        wait_for_slots_ready: false,
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters::default(),
            model: AgentDesiredModel::LocalToAgent("/nonexistent/model.gguf".to_owned()),
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
