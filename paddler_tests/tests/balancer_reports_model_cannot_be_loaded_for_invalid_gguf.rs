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
async fn balancer_reports_model_cannot_be_loaded_for_invalid_gguf() -> Result<()> {
    let invalid_gguf_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/invalid.gguf");

    let mut cluster = SubprocessCluster::start(SubprocessClusterParams {
        agent_count: 1,
        slots_per_agent: 1,
        wait_for_slots_ready: false,
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters::default(),
            model: AgentDesiredModel::LocalToAgent(invalid_gguf_path.to_owned()),
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

    let watch_agent_id = agent_id.clone();
    let expected_path = invalid_gguf_path.to_owned();

    cluster
        .agents
        .until(move |snapshot| {
            snapshot.agents.iter().any(|agent| {
                agent.id == watch_agent_id
                    && agent.issues.iter().any(|issue| {
                        matches!(issue, AgentIssue::ModelCannotBeLoaded(model_path)
                            if model_path.model_path == expected_path)
                    })
            })
        })
        .await
        .context("balancer should report ModelCannotBeLoaded for invalid GGUF fixture")?;

    cluster.shutdown().await?;

    Ok(())
}
