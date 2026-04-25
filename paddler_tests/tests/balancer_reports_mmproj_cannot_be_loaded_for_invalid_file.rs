#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use anyhow::Context as _;
use anyhow::Result;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::subprocess_cluster::SubprocessCluster;
use paddler_tests::subprocess_cluster_params::SubprocessClusterParams;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::agent_issue::AgentIssue;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::inference_parameters::InferenceParameters;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_reports_mmproj_cannot_be_loaded_for_invalid_file() -> Result<()> {
    let invalid_mmproj_path =
        concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/invalid_mmproj.gguf");

    let ModelCard { reference, .. } = qwen3_0_6b();

    let mut cluster = SubprocessCluster::start(SubprocessClusterParams {
        agent_count: 1,
        slots_per_agent: 1,
        wait_for_slots_ready: false,
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters::default(),
            model: AgentDesiredModel::HuggingFace(reference),
            multimodal_projection: AgentDesiredModel::LocalToAgent(invalid_mmproj_path.to_owned()),
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
    let expected_path = invalid_mmproj_path.to_owned();

    cluster
        .agents
        .until(move |snapshot| {
            snapshot.agents.iter().any(|agent| {
                agent.id == watch_agent_id
                    && agent.issues.iter().any(|issue| {
                        matches!(issue, AgentIssue::MultimodalProjectionCannotBeLoaded(model_path)
                            if model_path.model_path == expected_path)
                    })
            })
        })
        .await
        .context("balancer should report MultimodalProjectionCannotBeLoaded for invalid mmproj")?;

    cluster.shutdown().await?;

    Ok(())
}
